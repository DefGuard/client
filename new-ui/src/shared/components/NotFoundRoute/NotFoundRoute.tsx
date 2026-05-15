import { useRouter } from '@tanstack/react-router';

export const NotFoundRoute = () => {
  const router = useRouter();
  const availableRoutes = Object.keys(router.routesById);

  return (
    <div>
      <p>Route not found</p>
      <p>Detected: {window.location.href}</p>
      <p>Available routes:</p>
      <ul>
        {availableRoutes.map((route) => (
          <li key={route}>{route}</li>
        ))}
      </ul>
    </div>
  );
};
