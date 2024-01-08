// ease of use in rest of code while navigating, this should reflect what is built by createBrowserRouter in App.tsx
export const routes = {
  main: '/',
  client: {
    base: '/client',
    instancePage: '/client/instance',
    addInstance: '/client/add-instance',
    addTunnel: '/client/add-tunnel',
    tunnelCreated: '/client/tunnel-created',
    instanceCreated: '/client/instance-created',
    editTunnel: '/client/edit-tunnel',
    settings: '/client/settings',
    carousel: '/client/carousel',
  },
  enrollment: '/enrollment',
  timeout: '/timeout',
  passwordReset: '/password-reset',
};
