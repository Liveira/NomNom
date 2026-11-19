import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'inbox',
  name: 'Notifications',
  view: () => import('./View.svelte'),
});
