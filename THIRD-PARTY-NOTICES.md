# Third-party notices

OpenRideSync's original source code is licensed under MIT. This does not relicense
third-party software, map data, vehicle images, service content or trademarks.

- Tauri: MIT OR Apache-2.0 — https://github.com/tauri-apps/tauri
- React: MIT — https://github.com/facebook/react
- Vite: MIT — https://github.com/vitejs/vite
- Halfmoon 2.0.2 CSS: MIT, Copyright (c) 2023, Tahmid Khan.
  Its original license is retained in `src/vendor/halfmoon/LICENSE`.
- Leaflet 1.9.4: BSD-2-Clause — https://github.com/Leaflet/Leaflet
- Other Rust and npm dependencies retain the licenses declared in their packages.
  The build collects their distributed license texts in `THIRD-PARTY-LICENSES.txt`
  inside the portable ZIP, including build dependencies for completeness.
- The Google Firebase JavaScript SDK is loaded from Google's CDN during login;
  it is not bundled in the executable. https://github.com/firebase/firebase-js-sdk
- OpenStreetMap data © OpenStreetMap contributors, ODbL:
  https://www.openstreetmap.org/copyright
  Standard raster tiles: https://operations.osmfoundation.org/policies/tiles/

Vehicle descriptions and images are downloaded from the authenticated RoadSync
service for local viewing. They are not distributed with this repository or its
release packages. No personal Google credentials, account identity or saved
journeys are embedded by the build. Published releases include the service client
settings supplied through GitHub Actions Secrets at build time. Client keys are
extractable from the executable; source control contains only their SHA-256
fingerprints and the APK compatibility baseline in service-provenance.json.
