import { defineModule } from '../../lib/module-def';

export default defineModule({
  id: 'qr',
  name: 'QR codes',
  view: () => import('./View.svelte'),
});
