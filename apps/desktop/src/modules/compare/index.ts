import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'compare',
  name: 'Compare folders',
  view: () => import('./View.svelte'),
});
