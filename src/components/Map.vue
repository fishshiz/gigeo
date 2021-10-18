<script setup lang="ts">
import DrawerCarousel from './DrawerCarousel.vue';
import mapboxgl from 'mapbox-gl/dist/mapbox-gl.js';
import { onMounted, reactive, provide, ref } from 'vue'
import moment from 'moment';
import { GeocodeFeature, TMEvent, SpotifyArtist } from "../interface"
import { EVENT_TYPES } from "../constants"
interface State {
  map: any,
  events: TMEvent[],
  hoveredEvent: string,

}
const state: State = reactive({
  map: undefined,
  events: [],
  hoveredEvent: '',
});
const token = ref('')
provide('spotifyToken', token)
onMounted(() => {
  mapboxgl.accessToken = import.meta.env.VITE_MB_KEY;
  state.map = new mapboxgl.Map({
    container: 'map',
    style: 'mapbox://styles/fishshiz/ckukigrtdav6m17n3jfw2jx3j',
    center: [-96, 37.8], // starting position
    zoom: 3 // starting zoom
  });
  spotifySignIn();
  // Set an event listener for a specific layer
  state.map.on('click', 'events', (e: any) => {
    const element = document.getElementById(e.features[0].id);
    if (element) {
      element.scrollIntoView({ behavior: "smooth", block: "start", inline: "nearest" });
    }
  });

  // Change the cursor to a pointer when the it enters a feature in the 'circle' layer.
  state.map.on('mouseenter', 'events', () => {
    state.map.getCanvas().style.cursor = 'pointer';
  });

  // Change it back to a pointer when it leaves.
  state.map.on('mouseleave', 'events', () => {
    state.map.getCanvas().style.cursor = '';
  });
  if (navigator.geolocation) {
    navigator.geolocation.getCurrentPosition(setLocation);
  }
  function setLocation(position: GeolocationPosition): void {
    const latitude = position.coords.latitude;
    const longitude = position.coords.longitude;
    state.map.flyTo({
      center: [longitude, latitude],
      zoom: 9,
      speed: 0.8,
    });
  }
})

function markEvents(events: TMEvent[], drawLine: boolean = false) {
  clearMarkerState()
  const dataSource = { type: 'FeatureCollection', features: [] }
  let coordinates: [number, number][] = [];
  state.events = events.sort((a, b) => {
    const dateA = new Date(a.dates.start.dateTime).getTime();
    const dateB = new Date(b.dates.start.dateTime).getTime();
    return dateA - dateB;
  });
  events.forEach(place => {
    const venue = place._embedded.venues[0];

    if (!venue.location) {
      return;
    }
    const location = venue.location;

    const [longitude, latitude] = [
      parseFloat(location.longitude),
      parseFloat(location.latitude)
    ];
    coordinates.push([longitude, latitude]);
    const eventType: keyof typeof EVENT_TYPES = place.classifications ? place.classifications[0].segment.name : 'Music';
    const feature = {
      type: 'Feature',
      properties: {
        description: place.name,
        icon: EVENT_TYPES[eventType].icon,
        color: EVENT_TYPES[eventType].color,
        id: place.id,
      },
      geometry: {
        type: 'Point',
        coordinates: [longitude, latitude]
      }
    };
    //@ts-ignore
    dataSource.features.push(feature)
  });

  if (drawLine) {
    state.map.addSource('route', {
      'type': 'geojson',
      'lineMetrics': true,
      'data': {
        'type': 'Feature',
        'properties': {
        },
        'geometry': {
          'type': 'LineString',
          'coordinates': coordinates
        }
      }
    });
    state.map.addLayer({
      'id': 'route',
      'type': 'line',
      'source': 'route',
      'layout': {
        'line-cap': 'round'
      },
      'paint': {
        'line-color': '#888',
        'line-width': 4,
        'line-dasharray': [0, 2],
        'line-gradient': [
          'interpolate',
          ['linear'],
          ['line-progress'],
          0,
          'blue',
          0.1,
          'royalblue',
          0.3,
          'cyan',
          0.5,
          'lime',
          0.7,
          'yellow',
          1,
          'red'
        ]
      }
    });
  }

  let bounds = coordinates.reduce(function (bounds, coord) {
    return bounds.extend(coord);
  }, new mapboxgl.LngLatBounds(coordinates[0], coordinates[0]));
  state.map.fitBounds(bounds, {
    padding: 80
  });
  state.map.addSource('event-data', {
    'type': 'geojson',
    'promoteId': 'id',
    'data': dataSource
  })
  state.map.addLayer({
    'id': 'events',
    'type': 'symbol',
    'source': 'event-data',
    'layout': {
      'text-field': ['get', 'description'],
      'text-variable-anchor': ['top', 'bottom', 'left', 'right'],
      'text-radial-offset': 0.5,
      'text-justify': 'auto',
      'icon-image': ['get', 'icon'],
      'icon-size': 1.5,
    },
    'paint': {
      'icon-color': ['get', 'color'],
      'text-color': [
        'case',
        ['boolean', ['feature-state', 'hover'], false],
        '#627BC1',
        ['get', 'color']],
      'text-halo-color': '#9dc5bb',
      'text-halo-width': 1,
    }
  });
}

function clearMarkerState() {
  state.events = [];
  if (state.map.getLayer('events')) {
    state.map.removeLayer('events')
  }
  if (state.map.getSource('event-data')) {
    state.map.removeSource('event-data')
  }
  if (state.map.getLayer('route')) {
    state.map.removeLayer('route')
  }
  if (state.map.getSource('route')) {
    state.map.removeSource('route')
  }
}

async function handleArtistSearch(artist: SpotifyArtist) {
  const key = import.meta.env.VITE_TM_KEY
  const events = await getAllEvents(`https://app.ticketmaster.com/discovery/v2/events?apikey=${key}&keyword=${artist.name}`);
  markEvents(events, true)
}

async function handleCitySearch(obj: { geocode: GeocodeFeature, dateRange: [Date, Date] }) {
  const key = import.meta.env.VITE_TM_KEY
  const { geocode, dateRange } = obj;
  const dateStart = moment(dateRange[0]).startOf('day').format('YYYY-MM-DDTHH:mm:ss')
  const dateEnd = moment(dateRange[1]).endOf('day').format('YYYY-MM-DDTHH:mm:ss')
  const region = geocode.context.filter(context => context.id.startsWith('region'))[0];
  const country = geocode.context.filter(context => context.id.startsWith('country'))[0];

  const city = geocode.text;
  // SEATGEEK
  // const sgKey = import.meta.env.VITE_SG_KEY
  // fetch(
  //   `https://api.seatgeek.com/2/events?client_id=${sgKey}&datetime_utc.gte=${dateStart}Z&datetime_utc.lte=${dateEnd}Z&venue.city=${city}&venue.state=${region.short_code.slice(-2)}&venue.country=${country.short_code}`
  // ).then(response => response.json()).then(res => {
  //   markEvents(res._embedded.events)
  // });

  // TICKETMASTER
  const events = await getAllEvents(`https://app.ticketmaster.com/discovery/v2/events?apikey=${key}&startDateTime=${dateStart}Z&endDateTime=${dateEnd}Z&city=${city}&state=${region.text}&countryCode=${country.short_code}`);
  markEvents(events)
}

function handleHover(id: string | null) {
  if (id) {
    state.hoveredEvent = id;
    state.map.setFeatureState({
      source: 'event-data',
      id: id,
    }, {
      hover: true
    })
  } else {
    state.map.removeFeatureState({
      source: 'event-data',
      id: state.hoveredEvent,
    });
    state.hoveredEvent = "";
  }
}
async function getPage(url: string) {
  const results = await fetch(url).then(res => res.json());
  return results;
}

async function getAllEvents(url: string) {
  const firstResult = await getPage(url);
  if (!!firstResult._links.next) {
    const totalPages = firstResult.page.totalPages;
    const promises = [];
    for (let i = 0; i < totalPages - 1; i++) {
      promises.push(getPage(`${url}&page=${i + 1}&size=20`))
    }
    const results = await Promise.all(promises);
    return [firstResult._embedded.events, ...results].reduce((acc, d) => [...acc, ...d._embedded.events]);
  } else {
    return firstResult._embedded.events;
  }
};

function flyToItem(event: TMEvent) {
  state.map.flyTo({
    center: [event._embedded.venues[0].location.longitude, event._embedded.venues[0].location.latitude],
    speed: 0.8,
  });
}

function spotifySignIn() {
  const formBody = new FormData();
  formBody.set("grant_type", 'client_credentials');
  var scopes = 'user-read-private user-read-email';
  fetch('https://accounts.spotify.com/api/token?grant_type=client_credentials', {
    method: 'POST',


    // Adding headers to the request
    headers: {
      "Authorization": `Basic ${btoa(`${import.meta.env.VITE_SPOTIFY_ID}:${import.meta.env.VITE_SPOTIFY_SECRET}`)}`,
      "Content-type": "application/x-www-form-urlencoded"
    }
  }).then(res => {
    return res.json()
  }).then(resp => {
    token.value = resp.access_token;
  })
}
</script>

<template>
  <div id="map" />
  <DrawerCarousel
    @artist="handleArtistSearch"
    @geocode="handleCitySearch"
    @item-click="flyToItem"
    @hover="handleHover"
    :events="state.events"
  />
</template>

<style>
#map {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 100%;
  height: 100%;
}
</style>