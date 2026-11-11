<script lang="ts">
  let { size = 36 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 64 64" fill="none" role="img" aria-label="Vodka Launcher">
  <defs>
    <linearGradient id="vodka-logo" x1="14" y1="6" x2="50" y2="60" gradientUnits="userSpaceOnUse">
      <stop stop-color="#e9f7ff" />
      <stop offset=".55" stop-color="#9fdcff" />
      <stop offset="1" stop-color="#3fa9e6" />
    </linearGradient>
  </defs>
  <path
    d="M26 8h12v9.500c0 2.200.9 4.300 2.500 5.800C44 26.600 46 30.800 46 35.300V50a6 6 0 0 1-6 6H24a6 6 0 0 1-6-6V35.300c0-4.500 2-8.700 5.500-12 1.600-1.500 2.500-3.600 2.500-5.800V8Z"
    stroke="url(#vodka-logo)"
    stroke-width="3"
    stroke-linejoin="round"
    fill="rgba(164,222,255,0.06)"
  />
  <path d="M25 33.500 32 47l7-13.500" stroke="url(#vodka-logo)" stroke-width="3.400" stroke-linecap="round" stroke-linejoin="round" />
  <path d="M26 13.500h12" stroke="#e9f7ff" stroke-opacity=".55" stroke-width="2" />
</svg>
