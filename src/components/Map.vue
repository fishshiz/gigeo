<script setup lang="ts">
import DrawerCarousel from './DrawerCarousel.vue';
import mapboxgl from 'mapbox-gl/dist/mapbox-gl.js';
import { onMounted, reactive } from 'vue'
import moment from 'moment';
import { GeocodeResponse, GeocodeFeature, TMEvent } from "../interface"
import { EVENT_TYPES } from "../constants"
const state = reactive({
  map: null,
  events: [],
  hoveredEvent: '',
});
onMounted(() => {
  mapboxgl.accessToken = import.meta.env.VITE_MB_KEY;
  state.map = new mapboxgl.Map({
    container: 'map',
    style: 'mapbox://styles/fishshiz/ckukigrtdav6m17n3jfw2jx3j',
    center: [-96, 37.8], // starting position
    zoom: 3 // starting zoom
  });
  // Set an event listener for a specific layer
  state.map.on('click', 'events', (e) => {
    console.log('click: ', e.features);
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
  state.events = events;
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
    const eventType = place.classifications[0].segment.name;
    console.log(eventType, EVENT_TYPES[eventType])
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
}


async function handleCitySearch(obj: { geocode: GeocodeFeature, dateRange: [Date, Date] }) {
  console.log('handlesearch')
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
  const events = await getAllEvents(`https://app.ticketmaster.com/discovery/v2/events?apikey=${key}&startDateTime=${dateStart}Z&endDateTime=${dateEnd}Z&city=${city}&state=${region.text}&countryCode=${country.short_code}`);
  console.log(events)
  markEvents(events)
  // fetch(
  //   `https://app.ticketmaster.com/discovery/v2/events?apikey=${key}&startDateTime=${dateStart}Z&endDateTime=${dateEnd}Z&city=${city}&state=${region.text}&countryCode=${country.short_code}`
  // ).then(response => response.json()).then(res => {
  //   markEvents(res._embedded.events)
  // });
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
</script>

<template>
  <div id="map" />
  <DrawerCarousel @select="handleCitySearch" @hover="handleHover" :events="state.events" />
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