import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'recorder',
  name: 'Screen recorder',
  view: () => import('./View.svelte'),
});
