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
  <div class="theme-switch">
    <label id="switch" class="switch">
      <input type="checkbox" v-model="state.darkMode" id="slider" />
      <transition name="bounce">
        <span class="material-icons md-light" v-if="state.darkMode">&#xe51c;</span>
        <span class="material-icons md-dark" v-else>&#xe518;</span>
      </transition>
    </label>
  </div>
</template>

<style lang="scss">
html,
body {
  margin: 0;
  height: 100%;
}

#slider {
  display: none;
}

.theme-switch {
  position: absolute;
  right: 16px;
  top: 16px;
}

.material-icons.md-light {
  cursor: pointer;
  color: rgba(255, 255, 255, 1);
  color: #ffee10;
  text-shadow: 0 0 5px #ffee10;
}

.material-icons.md-dark {
  cursor: pointer;
  color: #273a50;
  text-shadow: 0 0 5px #273a50;
}

.material-icons:before {
  transform: scale(1);
}

.material-icons:hover {
  transition: 0.5s;
  transform: scale(1.2);
}

.bounce-enter-active {
  transform: scale(0);
  display: none;
  animation: bounce-in 0.5s;
}
.bounce-leave-active {
  animation: bounce-in 0.5s reverse;
}
@keyframes bounce-in {
  0% {
    transform: scale(0);
  }
  50% {
    transform: scale(1.25);
  }
  100% {
    transform: scale(1);
  }
}
</style>
