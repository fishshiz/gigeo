import { EVENT_TYPES } from "./constants";

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
  priceRanges: TMPriceRange[];
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
  genre: { id: string; name: keyof typeof EVENT_TYPES };
  primary: boolean;
  segment: { id: string; name: keyof typeof EVENT_TYPES };
  subGenre: { id: string; name: keyof typeof EVENT_TYPES };
  subType: { id: string; name: keyof typeof EVENT_TYPES };
  type: { id: string; name: keyof typeof EVENT_TYPES };
}

export interface TMImage {
  fallback: boolean;
  height: number;
  ratio: string;
  url: string;
  width: number;
}

export interface TMPriceRange {
  currency: string;
  max: number;
  min: number;
  type: string;
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
  location: Coordinates;
}

export interface Coordinates {
  latitude: string;
  longitude: string;
}

export interface SpotifyArtist {
  external_urls: {
    spotify: string;
  };
  followers: {
    href: string | null;
    total: number;
  };
  genres: string[];
  href: string;
  id: string;
  images: SpotifyArtistImage[];
  name: string;
  popularity: number;
  type: string;
  uri: string;
}

export interface SpotifyArtistImage {
  height: number;
  url: string;
  width: number;
}
