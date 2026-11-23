import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'mobile',
  name: 'Mobile Manager',
  view: () => import('./View.svelte'),
});
