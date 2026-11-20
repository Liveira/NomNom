import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'paths',
  name: 'PATH editor',
  view: () => import('./View.svelte'),
});
