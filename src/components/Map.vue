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
  console.log(state)
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
    console.log(position)
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
  clearMarkers()
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
    const el = document.createElement("div");
    el.className = "marker";
    const marker = new mapboxgl.Marker({ color: '#fff', element: el })
      .setLngLat([longitude, latitude])
      .addTo(state.map);
    state.markers.push(marker);
  });

  let bounds = coordinates.reduce(function (bounds, coord) {
    return bounds.extend(coord);
  }, new mapboxgl.LngLatBounds(coordinates[0], coordinates[0]));
  state.map.fitBounds(bounds, {
    padding: 80
  });
}

function clearMarkers() {
  state.markers.forEach(m => m.remove());
  state.markers = [];
}


function handleCitySearch(item: GeocodeFeature) {
  const key = import.meta.env.VITE_TM_KEY
  const dateNow = moment().format('YYYY-MM-DDTHH:mm:ss')
  const dateNext = moment().add(7, 'days').format('YYYY-MM-DDTHH:mm:ss')
  const state = item.context[1].text;
  const city = item.text;
  fetch(
    `https://app.ticketmaster.com/discovery/v2/events?apikey=${key}&startDateTime=${dateNow}Z&endDateTime=${dateNext}Z&city=${city}&state=${state}&countryCode=US`
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
  width: 1vw;
  height: 1vh;
  border-radius: 50%;
  background-color: #fff;
  box-shadow: 0 0 6vh 3vw #fff, 0 0 1vh 6vw #b400ff, 0 0 14vh 9vw #ff00d4;
}

.marker::after {
  background-color: rgba(255, 255, 255, 0.5);
  content: "";
  height: 50%;
  width: 15%;
  left: 18%;
  top: 0%;
  position: absolute;
  border-radius: 50%;
  transform: rotate(45deg);
}
</style>