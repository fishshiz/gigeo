//! The actual redesign: merges two `EventSource`s into one chronologically
//! ordered stream, instead of the old "stream Ticketmaster fully, then dump
//! PredictHQ at the end" shape. Fixes a real bug that shape caused: a
//! PredictHQ-only event's calendar day could land after Ticketmaster events
//! weeks later were already rendered in the (sorted, list-view) events
//! drawer, jolting it.
//!
//! Ticketmaster's per-window sequential stream keeps driving emission
//! exactly as before — fast time-to-first-event is preserved. Before each
//! window emits, this opportunistically checks whether PredictHQ's single
//! background fetch has resolved (see `predicthq::source::PredictHqSource`)
//! and, if so, reconciles + merges it in immediately; if not, the window
//! emits Ticketmaster-only and the day is queued for a later catch-up pass.
//! Fully monotonic for every window processed after PredictHQ resolves;
//! only windows already emitted before that point could still rarely see a
//! late insert — a much narrower version of the original problem, not a
//! new one.

use futures::Stream;

use super::reconcile::reconcile_predicthq_events;
use super::source::{EventSource, Window};
use super::types::EventResponse;
use crate::error::AppError;

/// One unit of output from `merge_events` — not yet serialized, not yet
/// enriched/personalized/artwork-backfilled. `events::stream` (the IO
/// layer) turns each of these into ndjson lines with the appropriate side
/// effects; this type and `merge_events` itself have none.
pub(crate) enum MergeStep {
    /// This window's Ticketmaster wave.
    Ticketmaster(Vec<EventResponse>),
    /// PredictHQ enrichments for one calendar day (clones of already-
    /// emitted Ticketmaster events with rank/predicted_attendance set).
    PredictHqEnrichments(Vec<EventResponse>),
    /// New PredictHQ-only events for one calendar day, pre-artwork-backfill.
    PredictHqNew(Vec<EventResponse>),
}

/// Merges `tm` and `phq` across `windows` in chronological order. See the
/// module docs for the ordering guarantee and its one accepted exception.
pub(crate) fn merge_events<TS, PS>(
    windows: Vec<Window>,
    mut tm: TS,
    mut phq: PS,
) -> impl Stream<Item = Result<MergeStep, AppError>>
where
    TS: EventSource,
    PS: EventSource,
{
    async_stream::try_stream! {
        // Ticketmaster windows whose PredictHQ reconciliation is still
        // outstanding, oldest first. Only ever non-empty while PredictHQ
        // hasn't resolved yet -- once it has, every window drains the
        // same pass it's pushed in (see below), so this never grows
        // unbounded in the common case.
        let mut pending: Vec<(Window, Vec<EventResponse>)> = Vec::new();

        for window in &windows {
            let tm_events = tm.events_for_window(window, false).await?.unwrap_or_default();
            yield MergeStep::Ticketmaster(tm_events.clone());
            pending.push((window.clone(), tm_events));

            // Opportunistic: drain as much of the backlog as PredictHQ is
            // ready for right now, oldest day first, stopping at the
            // first day it isn't ready for yet. If PredictHQ just became
            // ready, this clears the *entire* backlog in one pass before
            // the next window's Ticketmaster fetch even starts.
            let mut drained = 0;
            for (day_window, day_tm_events) in &pending {
                let Some(phq_events) = phq.events_for_window(day_window, false).await? else {
                    break;
                };
                let (enrichments, new_events) =
                    reconcile_predicthq_events(day_tm_events, phq_events);
                if !enrichments.is_empty() {
                    yield MergeStep::PredictHqEnrichments(enrichments);
                }
                if !new_events.is_empty() {
                    yield MergeStep::PredictHqNew(new_events);
                }
                drained += 1;
            }
            pending.drain(0..drained);
        }

        // Flush: force PredictHQ to resolve for any day still pending --
        // mirrors the old design's unconditional await after all
        // Ticketmaster windows finished, regardless of whether the
        // opportunistic checks above ever caught it ready.
        for (day_window, day_tm_events) in &pending {
            let phq_events = phq.events_for_window(day_window, true).await?.unwrap_or_default();
            let (enrichments, new_events) = reconcile_predicthq_events(day_tm_events, phq_events);
            if !enrichments.is_empty() {
                yield MergeStep::PredictHqEnrichments(enrichments);
            }
            if !new_events.is_empty() {
                yield MergeStep::PredictHqNew(new_events);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use futures::StreamExt;

    use super::*;
    use crate::events::types::{Performer, Source, VenueResponse};

    fn window(day: &str) -> Window {
        Window {
            start: format!("{day}T00:00:00Z"),
            end: format!("{day}T23:59:59Z"),
            calendar_day: day.to_string(),
        }
    }

    fn tm_event(id: &str, venue: &str, day: &str) -> EventResponse {
        EventResponse {
            id: id.to_string(),
            name: "Event".to_string(),
            venue: Some(VenueResponse {
                name: Some(venue.to_string()),
                location: None,
                city: None,
                state: None,
                state_code: None,
                address: None,
                postal_code: None,
                url: None,
                images: vec![],
            }),
            images: vec![],
            dates: Some(format!("{day}T23:00:00Z")),
            dates_pretty: None,
            local_calendar_day: Some(day.to_string()),
            classifications: None,
            performers: None,
            url: None,
            price_ranges: None,
            matched_artist: None,
            matched_via: None,
            source: Source::Ticketmaster,
            rank: None,
            predicted_attendance: None,
        }
    }

    fn phq_event(id: &str, venue: &str, local_day: &str) -> EventResponse {
        let mut e = tm_event(id, venue, local_day);
        e.source = Source::PredictHq;
        e.rank = Some(50);
        e.predicted_attendance = Some(100);
        e
    }

    fn with_performer(mut e: EventResponse, name: &str) -> EventResponse {
        e.performers = Some(vec![Performer {
            name: Some(name.to_string()),
            id: None,
            classifications: None,
            external_links: None,
            images: None,
            enrichment: None,
        }]);
        e
    }

    /// Always synchronously ready -- serves whatever's registered for a
    /// window's calendar day, same shape as the real `TicketmasterSource`
    /// from the caller's point of view.
    struct FakeTicketmasterSource {
        by_day: HashMap<String, Vec<EventResponse>>,
    }

    impl EventSource for FakeTicketmasterSource {
        async fn events_for_window(
            &mut self,
            window: &Window,
            _must_wait: bool,
        ) -> Result<Option<Vec<EventResponse>>, AppError> {
            Ok(Some(
                self.by_day
                    .get(&window.calendar_day)
                    .cloned()
                    .unwrap_or_default(),
            ))
        }
    }

    /// Controllable readiness: not ready until the `ready_after_calls`th
    /// invocation (or `must_wait`), then permanently ready -- mirrors the
    /// real `PredictHqSource`'s "one background fetch, resolves once"
    /// shape, but with a caller-controlled trigger instead of a real
    /// `JoinHandle`.
    struct FakePredictHqSource {
        by_day: HashMap<String, Vec<EventResponse>>,
        ready_after_calls: usize,
        calls: usize,
        ready: bool,
    }

    impl FakePredictHqSource {
        fn always_ready(by_day: HashMap<String, Vec<EventResponse>>) -> Self {
            Self {
                by_day,
                ready_after_calls: 0,
                calls: 0,
                ready: false,
            }
        }

        fn ready_after(by_day: HashMap<String, Vec<EventResponse>>, calls: usize) -> Self {
            Self {
                by_day,
                ready_after_calls: calls,
                calls: 0,
                ready: false,
            }
        }

        fn never_ready(by_day: HashMap<String, Vec<EventResponse>>) -> Self {
            Self {
                by_day,
                ready_after_calls: usize::MAX,
                calls: 0,
                ready: false,
            }
        }
    }

    impl EventSource for FakePredictHqSource {
        async fn events_for_window(
            &mut self,
            window: &Window,
            must_wait: bool,
        ) -> Result<Option<Vec<EventResponse>>, AppError> {
            self.calls += 1;
            if !self.ready {
                if must_wait || self.calls >= self.ready_after_calls {
                    self.ready = true;
                } else {
                    return Ok(None);
                }
            }
            Ok(Some(
                self.by_day
                    .get(&window.calendar_day)
                    .cloned()
                    .unwrap_or_default(),
            ))
        }
    }

    async fn run(
        windows: Vec<Window>,
        tm: FakeTicketmasterSource,
        phq: FakePredictHqSource,
    ) -> Vec<MergeStep> {
        let stream = merge_events(windows, tm, phq);
        futures::pin_mut!(stream);
        let mut out = Vec::new();
        while let Some(step) = stream.next().await {
            out.push(step.expect("fakes never error"));
        }
        out
    }

    fn label(step: &MergeStep) -> &'static str {
        match step {
            MergeStep::Ticketmaster(_) => "tm",
            MergeStep::PredictHqEnrichments(_) => "phq_enrich",
            MergeStep::PredictHqNew(_) => "phq_new",
        }
    }

    #[tokio::test]
    async fn phq_ready_before_first_window_fully_interleaves() {
        let windows = vec![window("2026-08-01"), window("2026-08-02")];
        let tm = FakeTicketmasterSource {
            by_day: HashMap::from([
                (
                    "2026-08-01".to_string(),
                    vec![tm_event("tm-1", "Venue A", "2026-08-01")],
                ),
                (
                    "2026-08-02".to_string(),
                    vec![tm_event("tm-2", "Venue B", "2026-08-02")],
                ),
            ]),
        };
        // Unmatched (different venue than any TM event that day) so each
        // day produces a PredictHqNew step, keeping the assertion simple.
        let phq = FakePredictHqSource::always_ready(HashMap::from([
            (
                "2026-08-01".to_string(),
                vec![phq_event("phq-1", "Other Venue", "2026-08-01")],
            ),
            (
                "2026-08-02".to_string(),
                vec![phq_event("phq-2", "Other Venue", "2026-08-02")],
            ),
        ]));

        let steps = run(windows, tm, phq).await;
        let labels: Vec<_> = steps.iter().map(label).collect();

        assert_eq!(labels, ["tm", "phq_new", "tm", "phq_new"]);
    }

    #[tokio::test]
    async fn phq_ready_mid_stream_triggers_catch_up_burst() {
        let windows: Vec<Window> = (1..=4).map(|d| window(&format!("2026-08-0{d}"))).collect();
        let tm = FakeTicketmasterSource {
            by_day: (1..=4)
                .map(|d| {
                    let day = format!("2026-08-0{d}");
                    (
                        day.clone(),
                        vec![tm_event(&format!("tm-{d}"), "Venue A", &day)],
                    )
                })
                .collect(),
        };
        let phq_by_day: HashMap<String, Vec<EventResponse>> = (1..=4)
            .map(|d| {
                let day = format!("2026-08-0{d}");
                (
                    day.clone(),
                    vec![phq_event(&format!("phq-{d}"), "Other Venue", &day)],
                )
            })
            .collect();
        // Call trace (see merge_events): window 1 and window 2's drain
        // loops each make exactly one (unready) PredictHQ call for the
        // oldest pending day -- calls #1 and #2. Window 3's drain loop
        // makes call #3 (still asking about the oldest pending day, "08-01"),
        // which is where readiness triggers -- so the whole backlog
        // (days 1, 2, 3) drains in that same pass, before window 4 runs.
        let phq = FakePredictHqSource::ready_after(phq_by_day, 3);

        let steps = run(windows, tm, phq).await;
        let labels: Vec<_> = steps.iter().map(label).collect();

        assert_eq!(
            labels,
            [
                "tm", "tm", "tm", "phq_new", "phq_new", "phq_new", "tm", "phq_new"
            ]
        );
    }

    #[tokio::test]
    async fn phq_never_ready_degrades_to_ticketmaster_only() {
        let windows = vec![window("2026-08-01"), window("2026-08-02")];
        let tm = FakeTicketmasterSource {
            by_day: HashMap::from([
                (
                    "2026-08-01".to_string(),
                    vec![tm_event("tm-1", "Venue A", "2026-08-01")],
                ),
                (
                    "2026-08-02".to_string(),
                    vec![tm_event("tm-2", "Venue B", "2026-08-02")],
                ),
            ]),
        };
        // never_ready still answers Some(empty) once forced via must_wait
        // during the final flush -- matching PredictHqSource::spawn(None)'s
        // real behavior (an already-resolved, empty cache) exactly.
        let phq = FakePredictHqSource::never_ready(HashMap::new());

        let steps = run(windows, tm, phq).await;
        let labels: Vec<_> = steps.iter().map(label).collect();

        assert_eq!(labels, ["tm", "tm"]);
    }

    #[tokio::test]
    async fn predicthq_only_event_never_precedes_its_own_windows_ticketmaster_wave() {
        // The regression test for the bug this redesign fixes. Day 1 has
        // a PredictHQ-only event (no matching Ticketmaster event that
        // day at all), but PredictHQ doesn't become ready until window 2
        // is being processed.
        let windows = vec![window("2026-08-01"), window("2026-08-02")];
        let tm = FakeTicketmasterSource {
            by_day: HashMap::from([(
                "2026-08-01".to_string(),
                vec![tm_event("tm-1", "Venue A", "2026-08-01")],
            )]),
        };
        let phq = FakePredictHqSource::ready_after(
            HashMap::from([(
                "2026-08-01".to_string(),
                vec![phq_event("phq-1", "Other Venue", "2026-08-01")],
            )]),
            2,
        );

        let steps = run(windows, tm, phq).await;

        let tm_day1_index = steps
            .iter()
            .position(|s| matches!(s, MergeStep::Ticketmaster(_)));
        let phq_new_index = steps
            .iter()
            .position(|s| matches!(s, MergeStep::PredictHqNew(_)));

        assert!(tm_day1_index.is_some());
        assert!(phq_new_index.is_some());
        assert!(
            tm_day1_index < phq_new_index,
            "day 1's PredictHQ-only event must never be yielded before day 1's Ticketmaster wave, got order {:?}",
            steps.iter().map(label).collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn utc_window_bucketing_matches_venue_local_reconcile_day() {
        // The real evening-show-crossing-midnight case this protects: both
        // sources' true UTC instant for this show falls on "2026-10-24"
        // (an 8pm local show, venue in a timezone behind UTC), so a real
        // Ticketmaster query for that UTC window is what returns this
        // event server-side, and a real PredictHqSource's own UTC-day
        // bucketing of its `dates` field independently lands it in the
        // same window -- even though both sources agree the show's
        // venue-local calendar day is "2026-10-23" (what
        // reconcile_predicthq_events actually matches on). This test
        // places both fake events under the window a real UTC-day
        // resolution would produce (crucially, NOT the window matching
        // local_calendar_day), and confirms merge_events still routes
        // them into the same reconcile call, which then matches on their
        // agreeing local_calendar_day despite the window/local-day split.
        let windows = vec![window("2026-10-23"), window("2026-10-24")];

        let mut tm_event = tm_event("tm-1", "State Theatre", "2026-10-24");
        tm_event.local_calendar_day = Some("2026-10-23".to_string());
        tm_event = with_performer(tm_event, "Guster");

        let mut phq_event = phq_event("phq-1", "State Theatre", "2026-10-24");
        phq_event.local_calendar_day = Some("2026-10-23".to_string());
        phq_event = with_performer(phq_event, "Guster");

        let tm = FakeTicketmasterSource {
            by_day: HashMap::from([("2026-10-24".to_string(), vec![tm_event])]),
        };
        let phq = FakePredictHqSource::always_ready(HashMap::from([(
            "2026-10-24".to_string(),
            vec![phq_event],
        )]));

        let steps = run(windows, tm, phq).await;
        let enrichment_count = steps
            .iter()
            .filter(|s| matches!(s, MergeStep::PredictHqEnrichments(_)))
            .count();
        let new_count = steps
            .iter()
            .filter(|s| matches!(s, MergeStep::PredictHqNew(_)))
            .count();

        assert_eq!(
            enrichment_count, 1,
            "expected the PredictHQ event to enrich its Ticketmaster match"
        );
        assert_eq!(new_count, 0, "expected no unmatched PredictHQ-only event");
    }
}
