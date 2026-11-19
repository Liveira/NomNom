import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'startup',
  name: 'Startup apps',
  view: () => import('./View.svelte'),
});
