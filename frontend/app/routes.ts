import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
  index("routes/home.tsx"),
  route("market", "routes/market-index.tsx"),
  route("market/:symbol", "routes/market-symbol.tsx"),
  route("strategies", "routes/projects-index.tsx"),
  route("strategies/:projectId", "routes/projects-detail.tsx"),
  route("strategies/:projectId/:tab", "routes/projects-stage.tsx"),
] satisfies RouteConfig;
