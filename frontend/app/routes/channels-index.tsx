import { redirect } from "react-router";

export function loader() {
  return redirect("/channels/general");
}

export default function ChannelsIndexRoute() {
  return null;
}
