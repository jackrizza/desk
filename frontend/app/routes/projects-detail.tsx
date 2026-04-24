import { useParams } from "react-router";
import ProjectsRoute from "./projects";

export default function ProjectsDetailRoute() {
  const params = useParams();

  return (
    <ProjectsRoute
      key={`${params.projectId ?? "detail"}:${params.tab ?? "build"}`}
      routeProjectId={params.projectId}
      routeTab={params.tab}
    />
  );
}
