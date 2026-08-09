import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import fs from 'fs';
import path from 'path';

// Vite plugin to serve mock_cdn files at /mock_cdn/
const serveMockCdn = () => ({
  name: 'serve-mock-cdn',
  configureServer(server) {
    server.middlewares.use((req, res, next) => {
      if (req.url && req.url.startsWith('/mock_cdn/')) {
        let fileName = path.basename(req.url.split('?')[0]);
        if (fileName === 'crm_module.html') {
          fileName = 'crm_dashboard.html';
        }
        const filePath = path.join(process.cwd(), 'mock_cdn', fileName);
        if (fs.existsSync(filePath)) {
          const content = fs.readFileSync(filePath);
          const ext = path.extname(filePath);
          const mime = ext === '.js' ? 'application/javascript' : ext === '.html' ? 'text/html' : 'text/plain';
          res.setHeader('Content-Type', mime);
          res.end(content);
          return;
        }
      }
      next();
    });
  }
});

export default defineConfig({
  plugins: [svelte(), serveMockCdn()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: true,
  },
});
