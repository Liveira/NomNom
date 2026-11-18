import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'monitor',
  name: 'System Monitor',
  view: () => import('./View.svelte'),
});
