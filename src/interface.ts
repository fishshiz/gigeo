export interface GeocodeFeature {
  bbox: number[];
  center: number[];
  context: { id: string; wikidata: string; text: string }[];
  geometry: { type: string; coordinates: number[] };
  id: string;
  place_name: string;
  place_type: string[];
  properties: { wikidata: string };
  relevance: number;
  text: string;
  type: string;
}

export interface GeocodeResponse {
  attribution: string;
  features: GeocodeFeature[];
  query: string[];
  type: string;
}
