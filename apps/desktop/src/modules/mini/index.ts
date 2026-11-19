import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'mini',
  name: 'Mini tools',
  view: () => import('./View.svelte'),
});
