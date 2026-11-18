import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'activity',
  name: 'Playtime',
  view: () => import('./View.svelte'),
});
