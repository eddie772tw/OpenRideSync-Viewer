import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { readFile } from 'node:fs/promises';
export default defineConfig({
  plugins: [react(), {
    name: 'explicit-local-preview',
    configureServer(server) {
      server.middlewares.use('/__preview/snapshot', async (_req,res) => {
        res.setHeader('Content-Type','application/json');
        res.setHeader('Cache-Control','no-store');
        if(process.env.ROADSYNC_PREVIEW !== '1'){res.statusCode=404;res.end('null');return;}
        try{res.end(await readFile('.preview/snapshot.json'));}catch{res.statusCode=404;res.end('null');}
      });
    },
  }],
  server: { port: 1420, strictPort: true },
  clearScreen: false,
});
