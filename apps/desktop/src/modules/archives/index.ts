import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'archives',
  name: 'ZIP files',
  view: () => import('./View.svelte'),
});
