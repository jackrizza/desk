import { useParams } from "react-router";
import MarketRoute from "./market";

export default function MarketIndexRoute() {
  const params = useParams();

  return (
    <MarketRoute
      key={`${params.symbol ?? "market-index"}`}
    />
  );
}
