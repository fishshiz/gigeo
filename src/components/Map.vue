<script setup lang="ts">
import DrawerCarousel from './DrawerCarousel.vue';
import mapboxgl from 'mapbox-gl/dist/mapbox-gl.js';
import { onMounted, reactive, provide, readonly, watch, ref } from 'vue'
import moment from 'moment';
import { GeocodeFeature, TMEvent, SpotifyArtist, Coordinates } from "../interface"
import { EVENT_TYPES, ICONS } from "../constants"
import { RateLimiter } from "limiter";
import Error from './Error.vue';
import { MapMouseEvent } from 'mapbox-gl';
import { activeGeocodeKey, updateActiveGeocodeKey, dateRangeKey, updateDateRangeKey, searchTermKey, updateSearchTermKey, spotifyTokenKey } from '../utils/injections';
interface Props {
  darkMode: boolean
}
const props = defineProps<Props>()
const darkMap = 'fishshiz/ckukigrtdav6m17n3jfw2jx3j';
const lightMap = 'fishshiz/ckv1v5li01lx514piujpqwkqa';
const drawerWidth = 408;
const spotifyToken = ref<string>('')
provide(spotifyTokenKey, readonly(spotifyToken))
const searchTerm = ref<string>('');
const updateSearchTerm = (t: string) => searchTerm.value = t;
provide(searchTermKey, searchTerm)
provide(updateSearchTermKey, updateSearchTerm);
const dateRange = ref<[Date, Date]>([new Date(), new Date()]);
const updateDateRange = (d: [Date, Date]) => dateRange.value = d;
provide(dateRangeKey, dateRange)
provide(updateDateRangeKey, updateDateRange)
const activeGeocode = ref<GeocodeFeature | undefined>()
const updateActiveGeocode = (g: GeocodeFeature | undefined) => activeGeocode.value = g;
provide(activeGeocodeKey, activeGeocode)
provide(updateActiveGeocodeKey, updateActiveGeocode)

watch(() => props.darkMode, (s, prevState) => {
  if (s) {
    switchBasemap(state.map, darkMap)
    // state.map.setStyle(darkMap);
  } else {
    switchBasemap(state.map, lightMap)
    // state.map.setStyle(lightMap);
  }
  // if (state.events.length) {
  //   console.log('hii', state.events)
  //   markEvents(state.events, false);
  // }
})

watch(() => activeGeocode.value, (s, prevState) => {
  if (typeof s !== undefined) {
    handleCitySearch()
  }
},
  { deep: true })

watch(() => dateRange.value, (d, prevState) => {
  handleCitySearch()
},
  { deep: true })

interface State {
  map: any,
  events: TMEvent[],
  hoveredEvent: string,
  selectedClusterEvents: TMEvent[],
  selectedClusterVenue: string,
  isError: boolean,
  errorMessage: string,
  selectedEvent: TMEvent | null,
  drawerOpen: boolean,
  loading: boolean,
}
const state: State = reactive({
  map: undefined,
  events: [],
  hoveredEvent: '',
  selectedClusterEvents: [],
  selectedClusterVenue: '',
  drawerOpen: true,
  isError: false,
  errorMessage: "",
  selectedEvent: null,
  loading: false,
});
onMounted(() => {
  mapboxgl.accessToken = import.meta.env.VITE_MB_KEY;
  const style = props.darkMode ? darkMap : lightMap;
  state.map = new mapboxgl.Map({
    container: 'map',
    style: `mapbox://styles/${style}`,
    center: [-96, 37.8], // starting position
    zoom: 3 // starting zoom
  });
  Promise.all(
    ICONS.map(img => new Promise<void>((resolve, reject) => {
      state.map.loadImage(`./src/icons/${img}.png`, ((error, res) => {
        console.log(img, res)
        state.map.addImage(img, res, { sdf: true })
        resolve();
      }))
    }))
  )
  spotifySignIn();

  // Set an event listener for a specific layer
  state.map.on('click', 'events', (e: any) => {
    console.log(e)
    const mapEvent = e.features[0];
    const location = { longitude: e.lngLat.lng, latitude: e.lngLat.lat };
    if (!mapEvent.properties.cluster_id) {
      const event = state.events.find(event => event.id === mapEvent.id);
      if (!!event) {
        state.selectedEvent = event;
      }
    } else {
      state.selectedClusterEvents = []
      state.selectedClusterVenue = mapEvent.properties.clusterVenue
      state.map.getSource('event-data').getClusterLeaves(e.features[0].properties.cluster_id, e.features[0].properties.points_count, 0, (error, features) => {
        console.log('Cluster leaves:', error, features);
        features.forEach((feature: any) => {
          const event = state.events.find(event => event.id === feature.properties.id);
          state.selectedClusterEvents.push((event as TMEvent));
        })
      })
    }
    if (!state.drawerOpen) {
      toggleDrawer()
    }
    flyToItem(location);
  });

  state.map.on('click', (e: MapMouseEvent) => {
    const features = state.map.queryRenderedFeatures(e.point, { layers: ['events'] });
    if (!features.length) {
      state.selectedClusterEvents = [];
      state.selectedClusterVenue = '';
      state.selectedEvent = null;
    }
  })
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

function clearSelection() {
  state.selectedEvent = null;
}

function clearCluster() {
  state.selectedClusterVenue = '';
  state.selectedClusterEvents = [];
}

// styleID should be in the form "mapbox/satellite-v9"
async function switchBasemap(map: any, styleID: string) {
  const newStyle = await fetch(
    `https://api.mapbox.com/styles/v1/${styleID}?access_token=${mapboxgl.accessToken}`
  ).then(res => res.json()).catch(e => setError(e));
  const currentStyle = map.getStyle();
  // ensure any sources from the current style are copied across to the new style
  newStyle.sources = Object.assign(
    {},
    currentStyle.sources,
    newStyle.sources
  );

  // find the index of where to insert our layers to retain in the new style
  let labelIndex = newStyle.layers.findIndex((el: any) => {
    return el.id == 'waterway-label';
  });

  // default to on top
  if (labelIndex === -1) {
    labelIndex = newStyle.layers.length;
  }
  const appLayers = currentStyle.layers.filter((el: any) => {
    // app layers are the layers to retain, and these are any layers which have a different source set
    return (
      el.source &&
      el.source != 'mapbox://mapbox.satellite' &&
      el.source != 'mapbox' &&
      el.source != 'composite'
    );
  });
  newStyle.layers = [
    ...newStyle.layers.slice(0, labelIndex),
    ...appLayers,
    ...newStyle.layers.slice(labelIndex, -1),
  ];
  map.setStyle(newStyle);
}

function markEvents(events: TMEvent[], drawLine: boolean = false) {
  clearMarkerState()
  if (!events.length) {
    state.events = [];
    return;
  }
  console.log('in marking', events)
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
        venue: place._embedded.venues[0].name,
        icon: EVENT_TYPES[eventType].icon,
        color: "#d5e68d",
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
  console.log('YAHOO', bounds)
  const goToPoint = state.map.project(bounds._sw)
  const adjustedSWCoords = state.map.unproject([goToPoint.x - (drawerWidth / 2), goToPoint.y])
  const adjustedBounds = new mapboxgl.LngLatBounds([adjustedSWCoords.lng, adjustedSWCoords.lat], [bounds._ne.lng, bounds._ne.lat]);
  console.log('adjustedBounds', adjustedBounds)
  console.log('adjustedSWCoords', adjustedSWCoords)
  console.log('goToPoint', goToPoint)
  state.map.fitBounds(adjustedBounds, {
    padding: 40,
    offset: [-(drawerWidth / 2), 0]
  });
  state.map.addSource('event-data', {
    'type': 'geojson',
    'promoteId': 'id',
    'cluster': true,
    'clusterRadius': 0,
    'clusterProperties': { "clusterEvent": ["string", ["get", "description"]], "clusterVenue": ["string", ["get", "venue"]] },
    'data': dataSource
  })
  state.map.addLayer({
    'id': 'events',
    'type': 'symbol',
    'source': 'event-data',
    'layout': {
      'text-field': [
        'case',
        ['boolean', ['has', 'point_count'], false],
        ['concat', ['get', 'point_count'], ' events at ', ['get', 'clusterVenue']],
        ['get', 'description'],
      ],
      'text-variable-anchor': ['top', 'bottom', 'left', 'right'],
      'text-radial-offset': 0.5,
      'text-justify': 'auto',
      'icon-image': [
        'case',
        ['boolean', ['has', 'icon'], false],
        ['get', 'icon'],
        'bullseye-solid'
      ],
      'icon-size': 0.6,
    },
    'paint': {
      'icon-color': '#d5e68d',
      'icon-halo-color': '#000',
      'icon-halo-width': 1,
      'text-color': [
        'case',
        ['boolean', ['feature-state', 'hover'], false],
        '#627BC1',
        ['boolean', ['has', 'point_count'], false],
        '#d5e68d',
        ['get', 'color'],],
      'text-halo-color': '#000',
      'text-halo-width': 1,
    }
  });
}

function clearMarkerState() {
  state.events = [];
  console.log('evemts: ', state.events)
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

async function handleArtistSearch(name: string) {
  searchTerm.value = name;
  state.loading = true;
  state.selectedEvent = null;
  const key = import.meta.env.VITE_TM_KEY
  const events = await getAllEvents(`https://app.ticketmaster.com/discovery/v2/events?apikey=${key}&keyword=${name}`);
  console.log(events);
  markEvents(events, true)
  state.loading = false;
}

async function handleCitySearch() {
  const geocode = activeGeocode.value
  state.loading = true;
  const key = import.meta.env.VITE_TM_KEY
  const dateStart = moment(dateRange.value[0]).startOf('day').format('YYYY-MM-DDTHH:mm:ss')
  const dateEnd = moment(dateRange.value[1]).endOf('day').format('YYYY-MM-DDTHH:mm:ss')
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
  state.loading = false;
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
async function getPage(url: string, limiter: any) {
  const remainingMessages = await limiter.removeTokens(1);
  console.log(console.log(limiter.getTokensRemaining()))
  const results = await fetch(url).then(res => res.json()).catch(e => setError(e));
  return results;
}

function setError(e: ErrorCallback) {
  console.log(e);
  state.isError = true;
  state.errorMessage = e.name;
}

const limiter = new RateLimiter({ tokensPerInterval: 5, interval: "second" });
async function getAllEvents(url: string) {
  const firstResult = await getPage(url, limiter);
  console.log(firstResult)
  if (!!firstResult._links.next) {
    const totalPages = firstResult.page.totalPages;
    const promises = [];
    for (let i = 0; i < totalPages - 1; i++) {
      promises.push(getPage(`${url}&page=${i + 1}&size=20`, limiter))
    }
    const results = await Promise.all(promises);
    return [firstResult._embedded.events, ...results].reduce((acc, d) => [...acc, ...d._embedded.events]);
  } else if (!!firstResult._embedded) {
    return firstResult._embedded.events;
  }
  return []
};

function handleItemClick(event: TMEvent) {
  const { location } = event._embedded.venues[0];
  state.selectedEvent = event;
  handleHover(event.id)
  flyToItem(location);
}

function flyToItem(location: Coordinates) {
  var w = window.innerWidth;
  var h = window.innerHeight;
  const goToPoint = state.map.project({ lng: location.longitude, lat: location.latitude })
  console.log(drawerWidth, Math.round(h / 2), goToPoint)
  const coords = state.map.unproject([goToPoint.x - (drawerWidth / 2), Math.round(h / 2)])
  state.map.easeTo({
    center: [coords.lng, location.latitude],
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
    spotifyToken.value = resp.access_token;
  }).catch(e => setError(e))
}

function toggleDrawer() {
  state.drawerOpen = !state.drawerOpen;
}
</script>

<template>
  <div id="map" />
  <DrawerCarousel
    @artist="handleArtistSearch"
    @item-click="handleItemClick"
    @hover="handleHover"
    @clear="clearSelection"
    @clear-cluster="clearCluster"
    @toggle="toggleDrawer"
    :events="state.events"
    :selected-events="state.selectedClusterEvents"
    :selected-venue="state.selectedClusterVenue"
    :drawer-open="state.drawerOpen"
    :selected-event="state.selectedEvent"
    :loading="state.loading"
  />
  <Error v-model:active="state.isError">
    <div class="error">{{ state.errorMessage }}</div>
  </Error>
</template>

<style>
#map {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 100%;
  height: 100%;
}

.error {
  color: var(--dynamic-border-color);
  font-size: 22px;
}
</style>