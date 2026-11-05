import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'settings',
  name: 'Settings',
  view: () => import('./View.svelte'),
});
