<script lang="ts">
  import Palette from '../app/Palette.svelte';
  import { launcherIpc } from '../lib/ipc';
</script>

<Palette floating onclose={() => void launcherIpc.close()} />
