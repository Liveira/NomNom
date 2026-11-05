import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'downloads',
  name: 'Downloads',
  view: () => import('./View.svelte'),
});
