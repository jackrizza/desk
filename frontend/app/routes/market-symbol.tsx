import { useParams } from "react-router";
import MarketRoute from "./market";

export default function MarketSymbolRoute() {
  const params = useParams();

  return (
    <MarketRoute
      key={`${params.symbol ?? "market-symbol"}`}
    />
  );
}
