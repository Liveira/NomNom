import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'notes',
  name: 'Notes',
  view: () => import('./View.svelte'),
});
