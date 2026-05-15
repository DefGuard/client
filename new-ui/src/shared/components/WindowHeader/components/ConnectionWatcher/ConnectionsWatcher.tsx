import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { api } from '../../../../rust-api/api';
import { isPresent } from '../../../../utils/isPresent';

export const ConnectionWatcher = () => {
  const { data: connections } = useQuery({
    queryKey: ['connections'],
    queryFn: api.getAllActiveConnections,
    refetchInterval: 5_000,
  });

  return (
    <div className="connection-watcher">
      {isPresent(connections) && <ul>{connections?.map((conn) => conn.name)}</ul>}
    </div>
  );
};
