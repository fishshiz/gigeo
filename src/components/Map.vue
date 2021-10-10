<script setup lang="ts">
import mapboxgl from 'mapbox-gl/dist/mapbox-gl.js';
 
mapboxgl.accessToken = 'pk.eyJ1IjoiZmlzaHNoaXoiLCJhIjoiY2pnd3FicGZ3MXNhajMzcDY3dnZxaDVjaSJ9.W2SIvSKLKbXVKOYWkV-3jQ';
var map = new mapboxgl.Map({
container: 'map',
style: 'mapbox://styles/fishshiz/ckukigrtdav6m17n3jfw2jx3j'
});
</script>

<template>
    
</template>

<style>
.mapboxgl-map {
  z-index: -1;
  width: 100%;
    height: 100%;
}
</style>