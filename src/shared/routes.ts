// ease of use in rest of code while navigating, this should reflect what is built by createBrowserRouter in App.tsx
export const routes = {
  main: '/',
  client: {
    base: '/client',
    instancePage: '/client/',
    addInstance: '/client/add-instance',
    addTunnel: '/client/add-tunnel',
    tunnelPage: '/client/tunnel',
    settings: '/client/settings',
  },
  enrollment: '/enrollment',
  timeout: '/timeout',
  passwordReset: '/password-reset',
};
