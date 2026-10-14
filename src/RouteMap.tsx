import {useEffect,useRef,useState} from 'react';
import L from 'leaflet';
import type {Trip} from './types';
import {km,time} from './format';
import 'leaflet/dist/leaflet.css';
export function RouteMap({trips,selected}:{trips:Trip[];selected:number}) {
  const node=useRef<HTMLDivElement>(null),map=useRef<L.Map|null>(null),layer=useRef<L.FeatureGroup|null>(null);
  const [failed,setFailed]=useState(false);
  const points=trips.reduce((n,t,i)=>n+(selected<0||selected===i?t.points.length:0),0);
  useEffect(()=>{
    if(!node.current)return;
    const m=L.map(node.current,{scrollWheelZoom:false});
    map.current=m;layer.current=L.featureGroup().addTo(m);
    const tiles=L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png',{
      maxZoom:19,referrerPolicy:'strict-origin-when-cross-origin',
      attribution:'&copy; <a href="https://www.openstreetmap.org/copyright" target="_blank" rel="noopener noreferrer">OpenStreetMap</a> contributors',
    }).addTo(m);
    tiles.on('tileerror',()=>setFailed(true));
    L.control.scale({imperial:false}).addTo(m);
    const observer=new ResizeObserver(()=>m.invalidateSize());observer.observe(node.current);
    return()=>{observer.disconnect();m.remove();map.current=null;layer.current=null;};
  },[]);
  useEffect(()=>{
    if(!map.current||!layer.current)return;
    const m=map.current,group=layer.current;group.clearLayers();
    const bounds=L.latLngBounds([]);
    const styles=getComputedStyle(document.documentElement);
    const colors=['--route-1','--route-2','--route-3'].map(k=>styles.getPropertyValue(k).trim());
    trips.forEach((trip,i)=>{
      if((selected>=0&&selected!==i)||!trip.points.length)return;
      const coords=trip.points.map(p=>[p[0],p[1]] as L.LatLngTuple);
      const line=L.polyline(coords,{color:colors[i%3],weight:4,opacity:0.95}).addTo(group);bounds.extend(line.getBounds());
      const tooltip=document.createElement('span');tooltip.textContent='第 '+(i+1)+' 段 · '+time(trip.start)+'–'+time(trip.end)+' · '+km(trip.distance)+' km';line.bindTooltip(tooltip);
      const markers:[L.LatLngTuple,string,string][]=[[coords[0],'起點','--route-start'],[coords[coords.length-1],'終點','--route-end']];
      markers.forEach(([point,label,color])=>{const text=document.createElement('span');text.textContent='第 '+(i+1)+' 段'+label;L.circleMarker(point,{radius:6,color:styles.getPropertyValue('--map-marker-border').trim(),weight:2,fillColor:styles.getPropertyValue(color).trim(),fillOpacity:1}).addTo(group).bindTooltip(text);});
    });
    if(bounds.isValid())m.fitBounds(bounds,{padding:[32,32],maxZoom:16});
    else m.setView([23.7,121],7);
  },[trips,selected]);
  return <div className="map-block"><div className="route-map" ref={node} role="region" aria-label="歷史旅程路線地圖"/>
    <div className="map-caption"><span><i className="start-dot"/> 起點 <i className="end-dot"/> 終點</span><span>{points.toLocaleString()} 個保存點位</span></div>
    {failed&&<p className="map-warning" role="status">街道底圖暫時無法載入，已保存的路線仍可檢視。恢復網路後重新開啟此分頁。</p>}
    {!points&&<p className="map-warning">此段旅程未提供座標。</p>}
  </div>;
}
