import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
  index("routes/home.tsx"),
  route("market", "routes/market-index.tsx"),
  route("market/:symbol", "routes/market-symbol.tsx"),
  route("projects", "routes/projects-index.tsx"),
  route("projects/:projectId", "routes/projects-detail.tsx"),
  route("projects/:projectId/:tab", "routes/projects-stage.tsx"),
] satisfies RouteConfig;
