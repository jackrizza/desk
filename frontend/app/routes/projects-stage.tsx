import { useParams } from "react-router";
import ProjectsRoute from "./projects";

export default function ProjectsStageRoute() {
  const params = useParams();

  return (
    <ProjectsRoute
      key={`${params.projectId ?? "stage"}:${params.tab ?? "build"}`}
      routeProjectId={params.projectId}
      routeTab={params.tab}
    />
  );
}
