// Download Romania's county (ADM1) boundaries from geoBoundaries (CC BY 4.0)
// into public/geo/romania-adm1.geojson. Run once: `npm run fetch-geo`.
import { writeFile, mkdir } from "node:fs/promises";

const meta = await (await fetch("https://www.geoboundaries.org/api/current/gbOpen/ROU/ADM1/")).json();
const url = meta.simplifiedGeometryGeoJSON || meta.gjDownloadURL;
if (!url) throw new Error("geoBoundaries API response had no GeoJSON URL");
const geo = await (await fetch(url)).text();
await mkdir("public/geo", { recursive: true });
await writeFile("public/geo/romania-adm1.geojson", geo);
console.log(`OK — saved ${geo.length} bytes from ${url}`);
console.log(`Licence: ${meta.boundaryLicense ?? "CC BY 4.0"}; source: ${meta.boundarySource ?? "geoBoundaries"}`);
