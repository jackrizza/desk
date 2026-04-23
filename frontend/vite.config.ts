import { reactRouter } from "@react-router/dev/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [tailwindcss(), reactRouter()],
  server: {
    host: true,
    proxy: {
      "/api": process.env.VITE_DEV_PROXY_TARGET ?? "http://localhost:3000",
    },
  },
  resolve: {
    tsconfigPaths: true,
  },
});
