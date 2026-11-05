<script lang="ts">
  import { app } from '../../app/state.svelte';

  const active = $derived(app.activeTasks);
</script>

<h1>Home</h1>

<section class="card">
  <h3>Active downloads</h3>
  {#if active.length === 0}
    <p class="empty">Nothing downloading.</p>
  {:else}
    <ul>
      {#each active as task (task.id)}
        <li>
          <span class="title">{task.title}</span>
          <span class="state tabular">
            {task.progress == null ? 'Waiting' : `${Math.round(task.progress * 100)}%`}
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  h1 { margin-bottom: 20px; }
  .card {
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    padding: 16px 18px;
    max-width: 640px;
  }
  h3 { margin-bottom: 10px; }
  .empty { margin: 0; color: var(--text-3); }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  li { display: flex; justify-content: space-between; gap: 12px; }
  .title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .state { color: var(--text-2); }
</style>
