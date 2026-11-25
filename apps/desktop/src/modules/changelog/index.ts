import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'changelog',
  name: 'Patch notes',
  view: () => import('./View.svelte'),
});
