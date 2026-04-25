import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
  index("routes/home.tsx"),
  route("market", "routes/market-index.tsx"),
  route("market/:symbol", "routes/market-symbol.tsx"),
  route("data-sources", "routes/data-sources.tsx"),
  route("channels/:channel?", "routes/channels.tsx"),
  route("strategies", "routes/projects-index.tsx"),
  route("strategies/:projectId", "routes/projects-detail.tsx"),
  route("strategies/:projectId/:tab", "routes/projects-stage.tsx"),
  route("traders", "routes/traders.tsx"),
  route("traders/:traderId", "routes/traders-detail.tsx"),
] satisfies RouteConfig;
