import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'gamemode',
  name: 'Game mode',
  view: () => import('./View.svelte'),
});
