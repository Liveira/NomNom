import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'home',
  name: 'Home',
  view: () => import('./View.svelte'),
});
