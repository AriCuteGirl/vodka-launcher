<script lang="ts">
  import PlaytimeList from "$lib/components/PlaytimeList.svelte";
  import { startLuduvo } from "$lib/luduvo.svelte";
  import { onMount } from "svelte";

  onMount(startLuduvo);
</script>

<PlaytimeList game="luduvo" />
