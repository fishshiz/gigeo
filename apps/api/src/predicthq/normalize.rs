//! Converting a raw PredictHQ event into this service's normalized
//! `EventResponse` shape — the one place that needs to know PredictHQ's raw
//! shape, mirroring `ticketmaster_stream::normalize`.

use super::client::{PhqLabel, PredictHqEvent};
use crate::events::types::{
    Classification, EventResponse, LocationResponse, Performer, Segment, Source, VenueResponse,
};
use chrono::{DateTime, Local};

pub(crate) fn normalize_predicthq_event(e: PredictHqEvent) -> EventResponse {
    let venue_entity = e.entities.iter().find(|en| en.entity_type == "venue");

    let geo = e.geo.as_ref();
    let venue = match (venue_entity, geo) {
        (None, None) => None,
        (venue_entity, geo) => Some(VenueResponse {
            name: venue_entity.map(|v| v.name.clone()),
            location: geo.map(|g| {
                let [lng, lat] = g.geometry.coordinates;
                LocationResponse {
                    latitude: Some(lat.to_string()),
                    longitude: Some(lng.to_string()),
                }
            }),
            city: geo.and_then(|g| g.address.as_ref()?.locality.clone()),
        }),
    };

    let performers: Vec<Performer> = e
        .entities
        .iter()
        .filter(|en| en.entity_type == "person" || en.entity_type == "organization")
        .map(|en| Performer {
            name: Some(en.name.clone()),
            id: Some(en.entity_id.clone()),
            classifications: None,
            external_links: None,
            images: None,
        })
        .collect();
    let performers = (!performers.is_empty()).then_some(performers);

    let dates_pretty = DateTime::parse_from_rfc3339(&e.start)
        .map(|dt| dt.with_timezone(&Local).format("%B %d").to_string())
        .ok();

    EventResponse {
        // PredictHQ and Ticketmaster ids are both opaque alphanumeric
        // strings from unrelated generators, but the collision risk costs
        // nothing to close off entirely with a source-prefixed id.
        id: format!("phq:{}", e.id),
        name: e.title,
        venue,
        images: vec![],
        dates: Some(e.start),
        dates_pretty,
        classifications: Some(build_classifications(e.phq_labels.as_deref())),
        performers,
        url: None,
        price_ranges: None,
        matched_artist: None,
        matched_via: None,
        source: Source::PredictHq,
        rank: Some(e.rank),
        predicted_attendance: e.phq_attendance,
    }
}

/// Maps PredictHQ's weighted `phq_labels` (e.g. `[{label: "rock", weight:
/// 0.77}, {label: "pop", weight: 0.23}]`) onto one `Classification` per
/// label — same shape as Ticketmaster's own multi-classification-with-a-
/// `primary`-flag list. Falls back to a bare `Music`-segment classification
/// (no genre) when PredictHQ has no labels for this event, which is common
/// for events lacking a structured performer entity too (same underlying
/// data gap) — the event still needs *a* classification so it groups
/// correctly under the existing "Music" filter.
fn build_classifications(labels: Option<&[PhqLabel]>) -> Vec<Classification> {
    let music_segment = || Segment {
        id: "predicthq-music".to_string(),
        name: "Music".to_string(),
    };

    let Some(labels) = labels.filter(|l| !l.is_empty()) else {
        return vec![Classification {
            primary: Some(true),
            segment: Some(music_segment()),
            genre: None,
            sub_genre: None,
            sub_type: None,
            family: Some(false),
        }];
    };

    let max_weight = labels.iter().map(|l| l.weight).fold(f64::MIN, f64::max);

    labels
        .iter()
        .map(|l| Classification {
            primary: Some(l.weight == max_weight),
            segment: Some(music_segment()),
            genre: Some(Segment {
                id: l.label.clone(),
                name: title_case_label(&l.label),
            }),
            sub_genre: None,
            sub_type: None,
            family: Some(false),
        })
        .collect()
}

/// `"jazz-and-classical"` -> `"Jazz And Classical"`.
fn title_case_label(label: &str) -> String {
    label
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicthq::client::{
        PredictHqAddress, PredictHqEntity, PredictHqGeo, PredictHqGeometry,
    };

    fn base_event() -> PredictHqEvent {
        PredictHqEvent {
            id: "abc123".to_string(),
            title: "Test Show".to_string(),
            rank: 50,
            phq_attendance: Some(300),
            entities: vec![],
            start: "2026-08-09T23:00:00Z".to_string(),
            geo: None,
            phq_labels: None,
        }
    }

    #[test]
    fn maps_id_with_source_prefix_and_source_field() {
        let normalized = normalize_predicthq_event(base_event());
        assert_eq!(normalized.id, "phq:abc123");
        assert_eq!(normalized.source, Source::PredictHq);
    }

    #[test]
    fn maps_rank_and_attendance() {
        let normalized = normalize_predicthq_event(base_event());
        assert_eq!(normalized.rank, Some(50));
        assert_eq!(normalized.predicted_attendance, Some(300));
    }

    #[test]
    fn has_no_url_price_ranges_or_images() {
        let normalized = normalize_predicthq_event(base_event());
        assert_eq!(normalized.url, None);
        assert_eq!(normalized.price_ranges, None);
        assert!(normalized.images.is_empty());
    }

    #[test]
    fn maps_person_and_organization_entities_to_performers_but_not_venue() {
        let mut event = base_event();
        event.entities = vec![
            PredictHqEntity {
                entity_id: "p1".to_string(),
                name: "Ryan Miller".to_string(),
                entity_type: "person".to_string(),
            },
            PredictHqEntity {
                entity_id: "o1".to_string(),
                name: "Guster".to_string(),
                entity_type: "organization".to_string(),
            },
            PredictHqEntity {
                entity_id: "v1".to_string(),
                name: "One Longfellow Square".to_string(),
                entity_type: "venue".to_string(),
            },
        ];

        let normalized = normalize_predicthq_event(event);
        let performers = normalized.performers.expect("expected performers");
        let names: Vec<_> = performers
            .iter()
            .filter_map(|p| p.name.as_deref())
            .collect();

        assert_eq!(names, vec!["Ryan Miller", "Guster"]);
        assert_eq!(
            normalized.venue.unwrap().name.as_deref(),
            Some("One Longfellow Square")
        );
    }

    #[test]
    fn no_performer_entities_yields_none_not_empty_vec() {
        let mut event = base_event();
        event.entities = vec![PredictHqEntity {
            entity_id: "v1".to_string(),
            name: "Some Venue".to_string(),
            entity_type: "venue".to_string(),
        }];

        let normalized = normalize_predicthq_event(event);
        assert_eq!(normalized.performers, None);
    }

    #[test]
    fn maps_venue_location_and_city_from_geo() {
        let mut event = base_event();
        event.geo = Some(PredictHqGeo {
            geometry: PredictHqGeometry {
                coordinates: [-70.39, 43.52],
            },
            address: Some(PredictHqAddress {
                locality: Some("Old Orchard Beach".to_string()),
            }),
        });

        let normalized = normalize_predicthq_event(event);
        let venue = normalized.venue.expect("expected venue");
        assert_eq!(venue.city.as_deref(), Some("Old Orchard Beach"));
        assert_eq!(venue.location.unwrap().latitude.as_deref(), Some("43.52"));
    }

    #[test]
    fn no_phq_labels_falls_back_to_bare_music_classification() {
        let normalized = normalize_predicthq_event(base_event());
        let classifications = normalized
            .classifications
            .expect("expected classifications");
        assert_eq!(classifications.len(), 1);
        assert_eq!(classifications[0].segment.as_ref().unwrap().name, "Music");
        assert!(classifications[0].genre.is_none());
        assert_eq!(classifications[0].primary, Some(true));
    }

    #[test]
    fn phq_labels_map_to_one_classification_each_with_highest_weight_primary() {
        let mut event = base_event();
        event.phq_labels = Some(vec![
            PhqLabel {
                label: "pop".to_string(),
                weight: 0.363,
            },
            PhqLabel {
                label: "rock".to_string(),
                weight: 0.364,
            },
            PhqLabel {
                label: "jazz-and-classical".to_string(),
                weight: 0.273,
            },
        ]);

        let normalized = normalize_predicthq_event(event);
        let classifications = normalized
            .classifications
            .expect("expected classifications");
        assert_eq!(classifications.len(), 3);

        let primary_count = classifications
            .iter()
            .filter(|c| c.primary == Some(true))
            .count();
        assert_eq!(primary_count, 1);

        let primary = classifications
            .iter()
            .find(|c| c.primary == Some(true))
            .unwrap();
        assert_eq!(primary.genre.as_ref().unwrap().name, "Rock");
    }

    #[test]
    fn title_cases_hyphenated_labels() {
        assert_eq!(title_case_label("jazz-and-classical"), "Jazz And Classical");
        assert_eq!(title_case_label("rock"), "Rock");
    }
}
