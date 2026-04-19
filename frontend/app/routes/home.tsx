import type { Route } from "./+types/home";
import { Homepage } from "../homepage/homepage";

export function meta({}: Route.MetaArgs) {
  return [
    { title: "Desk Manual Ops" },
    { name: "description", content: "Manual portfolio operations dashboard for the Desk API." },
  ];
}

export default function Home() {
  return <Homepage />;
}
