import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'clocks',
  name: 'Time tools',
  view: () => import('./View.svelte'),
});
