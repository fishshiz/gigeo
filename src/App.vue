<script setup lang="ts">
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup
import Map from './components/Map.vue'
import { reactive, onMounted, watch } from 'vue';
const state = reactive({
  darkMode: false,
})

watch(() => state.darkMode, (s, prevState) => {
  let htmlElement = document.documentElement;

  if (s) {
    localStorage.setItem("data-theme", 'dark');
    htmlElement.setAttribute('data-theme', 'dark');
  } else {
    localStorage.setItem("data-theme", 'light');
    htmlElement.setAttribute('data-theme', 'light');
  }
})
onMounted(() => {
  // check for active theme
  let htmlElement = document.documentElement;
  let theme = localStorage.getItem("data-theme");

  if (theme === 'dark') {
    htmlElement.setAttribute('data-theme', 'dark')
    state.darkMode = true
  } else {
    htmlElement.setAttribute('data-theme', 'light');
    state.darkMode = false
  }
})
</script>

<template>
  <Map :dark-mode="state.darkMode" />
  <input type="checkbox" class="theme-switch" v-model="state.darkMode" />
</template>

<style lang="scss">
html,
body {
  margin: 0;
  height: 100%;
}

.theme-switch {
  position: absolute;
  right: 0;
}
</style>
