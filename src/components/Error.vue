<script setup lang="ts">
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup
import { onMounted, watch } from 'vue';
interface Props {
    active: boolean,
}

const props = defineProps<Props>()
const emit = defineEmits<{
    (e: "update:active", val: boolean): void
}>()

onMounted(() => {
    if (props.active) {
        setActiveTimeout()
    }
})

watch(() => props.active, (val) => {
    console.log('ji')
    if (val) {
        setActiveTimeout()
    }
})

function setActiveTimeout() {
    setTimeout(() => {
        emit('update:active', false)
    }, 5000)
}
</script>

<template>
    <transition name="fade">
        <div class="error-wrapper" v-show="props.active">
            <div class="error-container">
                <span class="material-icons error">&#xe002;</span>
                <slot class="slot"></slot>
            </div>
        </div>
    </transition>
</template>

<style lang="scss">
.error-wrapper {
    position: absolute;
    bottom: 16px;
    background: var(--dynamic-subtitle-color);
    right: 16px;
    width: 400px;
    height: 100px;
    padding: 16px;
    border-radius: 12px;
}

.error-container {
    position: relative;
}

.material-icons.error {
    color: rgba(255, 255, 255, 1);
    color: #f6a38e;
    text-shadow: 0 0 1px #f9b4a3;
    position: absolute;
    top: 16px;
    left: 16px;
}

.slot {
    position: absolute;
    top: 16px;
    left: 32px;
}
.fade-enter-active,
.fade-leave-active {
    transition: all 0.5s ease;
}

.fade-enter-from,
.fade-leave-to {
    transform: translateY(200px);
    opacity: 0;
}
</style>
