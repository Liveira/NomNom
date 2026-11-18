import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'tools',
  name: 'Tools',
  view: () => import('./View.svelte'),
});
