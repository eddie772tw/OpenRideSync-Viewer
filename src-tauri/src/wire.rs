//! The measured APK schema is reused; this transport has no write method.
use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage, MessageDescriptor, SerializeOptions};
use prost_types::{DescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileDescriptorSet};
use serde_json::Value;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};

pub const METHODS: &[&str] = &[
    "ListModelCatalogs",
    "ListModelSeries",
    "ListModels",
    "GetImageBinary",
    "ListDailyTrips",
    "ListTripDetails",
];
pub fn pool() -> Result<DescriptorPool, String> {
    let schema: Value =
        serde_json::from_str(include_str!("../wire-schema.json")).map_err(|_| "SCHEMA_INVALID")?;
    let mut messages = Vec::new();
    for (name, fields) in schema.as_object().ok_or("SCHEMA_INVALID")? {
        let mut message = DescriptorProto {
            name: Some(name.clone()),
            ..Default::default()
        };
        for field in fields.as_array().ok_or("SCHEMA_INVALID")? {
            let kind = field["type"].as_str().ok_or("SCHEMA_INVALID")?;
            let number = match kind {
                "double" => 1,
                "float" => 2,
                "int64" => 3,
                "uint64" => 4,
                "int32" => 5,
                "bool" => 8,
                "string" => 9,
                "bytes" => 12,
                "uint32" => 13,
                "sint32" => 17,
                _ => 11,
            };
            message.field.push(FieldDescriptorProto {
                name: Some(field["name"].as_str().ok_or("SCHEMA_INVALID")?.into()),
                number: Some(field["number"].as_i64().ok_or("SCHEMA_INVALID")? as i32),
                label: Some(if field["repeated"] == true { 3 } else { 1 }),
                r#type: Some(number),
                type_name: (number == 11).then(|| format!(".viewer.{kind}")),
                ..Default::default()
            });
        }
        messages.push(message);
    }
    DescriptorPool::from_file_descriptor_set(FileDescriptorSet {
        file: vec![FileDescriptorProto {
            name: Some("viewer.proto".into()),
            package: Some("viewer".into()),
            syntax: Some("proto3".into()),
            message_type: messages,
            ..Default::default()
        }],
    })
    .map_err(|_| "SCHEMA_INVALID".into())
}

pub fn descriptor(pool: &DescriptorPool, name: &str) -> Result<MessageDescriptor, String> {
    pool.get_message_by_name(&format!("viewer.{name}"))
        .ok_or("SCHEMA_TYPE_MISSING".into())
}
pub fn request(
    pool: &DescriptorPool,
    method: &str,
    fields: Value,
) -> Result<DynamicMessage, String> {
    if !METHODS.contains(&method) {
        return Err("READ_ONLY_METHOD_REQUIRED".into());
    }
    DynamicMessage::deserialize(descriptor(pool, &format!("{method}Request"))?, fields)
        .map_err(|_| "REQUEST_FIELDS_INVALID".into())
}
fn unknown(message: &DynamicMessage) -> bool {
    use prost_reflect::Value as P;
    fn nested(value: &P) -> bool {
        match value {
            P::Message(m) => unknown(m),
            P::List(v) => v.iter().any(nested),
            P::Map(m) => m.values().any(nested),
            _ => false,
        }
    }
    message.unknown_fields().next().is_some() || message.fields().any(|(_, v)| nested(v))
}
pub fn json(message: &DynamicMessage) -> Result<Value, String> {
    if unknown(message) {
        return Err("UNKNOWN_WIRE_FIELDS".into());
    }
    let mut bytes = Vec::new();
    message
        .serialize_with_options(
            &mut serde_json::Serializer::new(&mut bytes),
            &SerializeOptions::new().skip_default_fields(false),
        )
        .map_err(|_| "RESPONSE_INVALID")?;
    serde_json::from_slice(&bytes).map_err(|_| "RESPONSE_INVALID".into())
}
pub struct DynamicCodec(pub MessageDescriptor);
pub struct DynamicEncoder;
pub struct DynamicDecoder(pub MessageDescriptor);
impl Codec for DynamicCodec {
    type Encode = DynamicMessage;
    type Decode = DynamicMessage;
    type Encoder = DynamicEncoder;
    type Decoder = DynamicDecoder;
    fn encoder(&mut self) -> Self::Encoder {
        DynamicEncoder
    }
    fn decoder(&mut self) -> Self::Decoder {
        DynamicDecoder(self.0.clone())
    }
}
impl Encoder for DynamicEncoder {
    type Item = DynamicMessage;
    type Error = tonic::Status;
    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        item.encode(dst)
            .map_err(|_| tonic::Status::internal("ENCODE_FAILED"))
    }
}
impl Decoder for DynamicDecoder {
    type Item = DynamicMessage;
    type Error = tonic::Status;
    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        DynamicMessage::decode(self.0.clone(), src)
            .map(Some)
            .map_err(|_| tonic::Status::internal("DECODE_FAILED"))
    }
}
pub struct Client {
    channel: tonic::transport::Channel,
    pool: DescriptorPool,
    key: String,
}
impl Client {
    pub async fn new(key: String) -> Result<Self, String> {
        let channel = tonic::transport::Endpoint::from_static("https://api.honda-roadsync.com")
            .tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())
            .map_err(|_| "TLS_CONFIG_FAILED")?
            .connect_timeout(std::time::Duration::from_secs(20))
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("RoadSync/26.8.10;309 android/36")
            .map_err(|_| "CLIENT_CONFIG_FAILED")?
            .connect()
            .await
            .map_err(|_| "CLOUD_UNREACHABLE")?;
        Ok(Self {
            channel,
            pool: pool()?,
            key,
        })
    }
    pub async fn call(&self, method: &str, fields: Value, token: &str) -> Result<Value, String> {
        let message = request(&self.pool, method, fields)?;
        let response_name = if method == "GetImageBinary" {
            "ImageBinary".into()
        } else {
            format!("{method}Response")
        };
        let codec = DynamicCodec(descriptor(&self.pool, &response_name)?);
        let mut req = tonic::Request::new(message);
        req.metadata_mut().insert(
            "authorization",
            format!("Bearer {token}")
                .parse()
                .map_err(|_| "AUTH_INVALID")?,
        );
        req.metadata_mut().insert(
            "x-api-key",
            self.key.parse().map_err(|_| "SERVICE_CONFIG_INVALID")?,
        );
        req.set_timeout(std::time::Duration::from_secs(25));
        let path = format!("/com.honda.roadsync.api.v2.Service/{method}")
            .parse()
            .map_err(|_| "METHOD_INVALID")?;
        let mut grpc = tonic::client::Grpc::new(self.channel.clone())
            .max_decoding_message_size(16 * 1024 * 1024);
        grpc.ready().await.map_err(|_| "CLOUD_UNREACHABLE")?;
        let response = grpc
            .unary(req, path, codec)
            .await
            .map_err(|status| format!("CLOUD_RPC_{:?}", status.code()))?;
        json(&response.into_inner())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn measured_request_and_write_rejection() {
        let p = pool().unwrap();
        let r = request(
            &p,
            "ListDailyTrips",
            serde_json::json!({"offsetFromUtc":28800,"startDate":"1","tripsCount":30}),
        )
        .unwrap();
        assert_eq!(r.encode_to_vec(), vec![8, 128, 194, 3, 16, 1, 24, 30]);
        for method in [
            "CreateLocation",
            "DeleteTrip",
            "DeleteMultiTrips",
            "../ListDailyTrips",
        ] {
            assert!(request(&p, method, serde_json::json!({})).is_err());
        }
        let r = DynamicMessage::decode(
            descriptor(&p, "ListDailyTripsResponse").unwrap(),
            &[0xf8, 7, 1][..],
        )
        .unwrap();
        assert!(json(&r).is_err());
    }
    #[test]
    #[ignore = "Requires an explicitly selected private verified measurement run"]
    fn replay_measured_responses() {
        use sha2::{Digest, Sha256};
        let root = std::path::PathBuf::from(
            std::env::var("ROADSYNC_VERIFIED_RUN").expect("Set ROADSYNC_VERIFIED_RUN"),
        );
        let verified: Value =
            serde_json::from_slice(&std::fs::read(root.join("VERIFIED-SUMMARY.json")).unwrap())
                .unwrap();
        let manifest_bytes = std::fs::read(root.join("verified-artifacts.json")).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&manifest_bytes)),
            verified["artifact_manifest_sha256"]
        );
        let manifest: Vec<Value> = serde_json::from_slice(&manifest_bytes).unwrap();
        let pool = pool().unwrap();
        let mut count = 0;
        for entry in manifest {
            let relative = entry["path"].as_str().unwrap().replace('\\', "/");
            if !relative.ends_with(".response.bin") {
                continue;
            }
            let path = root.join(&relative).canonicalize().unwrap();
            assert!(path.starts_with(root.canonicalize().unwrap()));
            let bytes = std::fs::read(&path).unwrap();
            assert_eq!(format!("{:x}", Sha256::digest(&bytes)), entry["sha256"]);
            assert_eq!(bytes[0], 0);
            assert_eq!(
                u32::from_be_bytes(bytes[1..5].try_into().unwrap()) as usize,
                bytes.len() - 5
            );
            let method = path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .trim_end_matches(".response.bin");
            let message = DynamicMessage::decode(
                descriptor(&pool, &format!("{method}Response")).unwrap(),
                &bytes[5..],
            )
            .unwrap();
            let actual = json(&message).unwrap();
            let mut expected: Value = serde_json::from_slice(
                &std::fs::read(path.with_file_name(format!("{method}.decoded.json"))).unwrap(),
            )
            .unwrap();
            expected.as_object_mut().unwrap().remove("_unknownFields");
            expected
                .as_object_mut()
                .unwrap()
                .remove("_unknownFieldNumbers");
            assert!(
                actual == expected,
                "Measured response decode differs for {method}"
            );
            count += 1;
        }
        assert_eq!(count, 77);
        println!("77 measured HTTP/2 gRPC replies decoded identically; no network requests");
    }
}
