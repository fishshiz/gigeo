<script setup lang="ts">
import SearchWrapper from './SearchWrapper.vue'
import mapboxgl from 'mapbox-gl/dist/mapbox-gl.js';
import { onMounted, reactive } from 'vue'
import moment from 'moment';
import { GeocodeResponse, GeocodeFeature, TMEvent } from "../interface"

const state = reactive({
  map: null,
  markers: [],
});
onMounted(() => {
  mapboxgl.accessToken = import.meta.env.VITE_MB_KEY;
  state.map = new mapboxgl.Map({
    container: 'map',
    style: 'mapbox://styles/fishshiz/ckukigrtdav6m17n3jfw2jx3j',
    center: [-96, 37.8], // starting position
    zoom: 3 // starting zoom
  });
  state.map.loadImage('../src/icons/music_note.png', (error, image) => {
    if (error) throw error;
    state.map.addImage('music-note', image);
  })
  if (navigator.geolocation) {
    navigator.geolocation.getCurrentPosition(setLocation);
  }
  function setLocation(position: PositionCallback) {
    const latitude = position.coords.latitude;
    const longitude = position.coords.longitude;
    state.map.flyTo({
      center: [longitude, latitude],
      zoom: 9,
      speed: 0.8,
    });
  }
})

function markEvents(events: TMEvent[]) {
  clearMarkerState()
  const dataSource = { type: 'FeatureCollection', features: [] }
  let coordinates: [number, number][] = [];
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
    const feature = {
      type: 'Feature',
      properties: {
        description: place.name,
        icon: 'american-football'
      },
      geometry: {
        type: 'Point',
        coordinates: [longitude, latitude]
      }
    };
    dataSource.features.push(feature)
  });

  let bounds = coordinates.reduce(function (bounds, coord) {
    return bounds.extend(coord);
  }, new mapboxgl.LngLatBounds(coordinates[0], coordinates[0]));
  state.map.fitBounds(bounds, {
    padding: 80
  });
  state.map.addSource('event-data', {
    'type': 'geojson',
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
      'icon-image': ['get', 'icon']
    },
    'paint': {
      'text-color': '#fff'
    }
  });
}

function clearMarkerState() {
  state.markers.forEach(m => m.remove());
  state.markers = [];
  if (state.map.getLayer('events')) {
    state.map.removeLayer('events')
  }
  if (state.map.getSource('event-data')) {
    state.map.removeSource('event-data')
  }
}


function handleCitySearch(obj: { geocode: GeocodeFeature, dateRange: [Date, Date] }) {
  const key = import.meta.env.VITE_TM_KEY
  const { geocode, dateRange } = obj;
  const dateStart = moment(dateRange[0]).startOf('day').format('YYYY-MM-DDTHH:mm:ss')
  const dateEnd = moment(dateRange[1]).endOf('day').format('YYYY-MM-DDTHH:mm:ss')
  const region = geocode.context.filter(context => context.id.startsWith('region'))[0];
  const country = geocode.context.filter(context => context.id.startsWith('country'))[0];

  const city = geocode.text;
  fetch(
    `https://app.ticketmaster.com/discovery/v2/events?apikey=${key}&startDateTime=${dateStart}Z&endDateTime=${dateEnd}Z&city=${city}&state=${region.text}&countryCode=${country.short_code}`
  ).then(response => response.json()).then(res => {

    markEvents(res._embedded.events)
  });
}
</script>

<template>
  <div id="map" />
  <SearchWrapper @select="handleCitySearch" />
</template>

<style>
#map {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 100%;
  height: 100%;
}

.marker {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background-color: #fff;
  box-shadow: inset 0 0 5px #567, inset 2px 0 8px #1a2637a1,
    inset -2px 0 8px #0ff, inset 2px 0 30px #081217, inset -2px 0 30px #0ff,
    0 0 5px #5566779e, -1px 0 8px #081217, 1px 0 8px #0ff;
}
</style>