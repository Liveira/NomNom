import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'shots',
  name: 'Screenshots',
  view: () => import('./View.svelte'),
});
