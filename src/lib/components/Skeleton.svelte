<script lang="ts">
  let { width = "100%", height = "1em", radius = "var(--r-sm)" }: { width?: string; height?: string; radius?: string } = $props();
</script>

<span class="skeleton" style:width style:height style:border-radius={radius} aria-hidden="true"></span>

<style>
  .skeleton {
    display: block;
    background: linear-gradient(90deg, var(--glass) 25%, var(--glass-strong) 50%, var(--glass) 75%);
    background-size: 200% 100%;
    animation: shimmer 1.4s linear infinite;
  }
</style>
