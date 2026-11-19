import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'phone',
  name: 'Phone transfer',
  view: () => import('./View.svelte'),
});
