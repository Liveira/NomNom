import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'actions',
  name: 'My actions',
  view: () => import('./View.svelte'),
});
