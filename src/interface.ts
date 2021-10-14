export interface GeocodeFeature {
  bbox: number[];
  center: number[];
  context: { id: string; short_code: string; wikidata: string; text: string }[];
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

export interface TMEvent {
  accessibility: { ticketLimit: number };
  ageRestrictions: { legalAgeEnforced: boolean };
  classifications: TMClassification[];
  dates: {
    start: {
      dateTBA: boolean;
      dateTBD: boolean;
      dateTime: string;
      localDate: string;
      localTime: string;
      noSpecificTime: boolean;
      timeTBA: boolean;
    };
    timezone: string;
    status: { code: string };
    spanMultipleDays: boolean;
  };
  id: string;
  images: TMImage[];
  info: string;
  locale: string;
  name: string;
  pleaseNote: string;
  priceRanges: { currency: string; max: number; min: number; type: string }[];
  products: TMProduct[];
  promoter: TMPromoter;
  promoters: TMPromoter[];
  sales: {
    public: {
      endDateTime: string;
      startDateTime: string;
      startTBA: boolean;
      startTBD: boolean;
    };
    presales: { endDateTime: string; name: string; startDateTime: string }[];
  };
  seatmap: { staticUrl: string };
  test: false;
  ticketLimit: { info: string };
  ticketing: {
    healthCheck: { description: string; learnMoreUrl: string; summary: string };
    safeTix: { enabled: boolean };
  };
  type: string;
  url: string;
  _embedded: {
    venues: TMVenue[];
  };
}

interface TMClassification {
  family: boolean;
  genre: { id: string; name: string };
  primary: boolean;
  segment: { id: string; name: string };
  subGenre: { id: string; name: string };
  subType: { id: string; name: string };
  type: { id: string; name: string };
}

export interface TMImage {
  fallback: boolean;
  height: number;
  ratio: string;
  url: string;
  width: number;
}

interface TMProduct {
  classifications: TMClassification[];
  id: string;
  name: string;
  type: string;
  url: string;
}

interface TMPromoter {
  id: string;
  name: string;
  description: string;
}

export interface TMVenue {
  address: { line1: string };
  city: { name: string };
  country: { name: string; countryCode: string };
  dmas: { id: number }[];
  id: string;
  locale: string;
  name: string;
  postalCode: string;
  state: { name: string; stateCode: string };
  test: boolean;
  timezone: string;
  type: string;
  location: {
    latitude: string;
    longitude: string;
  };
}
