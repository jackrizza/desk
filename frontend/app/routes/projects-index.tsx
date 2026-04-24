import { useParams } from "react-router";
import ProjectsRoute from "./projects";

export default function ProjectsIndexRoute() {
  const params = useParams();

  return (
    <ProjectsRoute
      key={`${params.projectId ?? "index"}:${params.tab ?? "build"}`}
      routeProjectId={params.projectId}
      routeTab={params.tab}
    />
  );
}
