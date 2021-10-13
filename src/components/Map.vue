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
        icon: 'music',
        color: '#d7335c'
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
      'icon-image': ['get', 'icon'],
      'icon-size': 1.5
    },
    'paint': {
      'text-color': ['get', 'color'],
      'text-halo-color': '#9dc5bb',
      'text-halo-width': 4,
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
  //   debugger;
  //   markEvents(res._embedded.events)
  // });

  // TICKETMASTER
  const events = await getFullList(`https://app.ticketmaster.com/discovery/v2/events?apikey=${key}&startDateTime=${dateStart}Z&endDateTime=${dateEnd}Z&city=${city}&state=${region.text}&countryCode=${country.short_code}`, 0);
  console.log(events)
  markEvents(events)
  // fetch(
  //   `https://app.ticketmaster.com/discovery/v2/events?apikey=${key}&startDateTime=${dateStart}Z&endDateTime=${dateEnd}Z&city=${city}&state=${region.text}&countryCode=${country.short_code}`
  // ).then(response => response.json()).then(res => {
  //   markEvents(res._embedded.events)
  // });
}
async function getUsers(url: string, pageNo = 1) {

  let actualUrl = pageNo ? url + `?page=${pageNo}&limit=20` : url;
  var apiResults = await fetch(actualUrl)
    .then(resp => {
      return resp.json();
    });

  return apiResults;

}
async function getFullList(url: string, pageNo = 0) {
  const results = await getUsers(url, pageNo);
  console.log("Retreiving data from API for page : " + pageNo, results);
  if (results._links.next) {
    return results._embedded.events.concat(await getFullList(url, pageNo + 1));
  } else {
    return results;
  }
};
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
</style>