import byteSize from 'byte-size';
import classNames from 'classnames';
import dayjs from 'dayjs';
import { floor, isUndefined } from 'lodash-es';
import { ReactNode, useCallback, useEffect, useMemo, useRef } from 'react';

import { useI18nContext } from '../../../../../../../../../../../i18n/i18n-react';
import {
  ListHeader,
  ListRowCell,
  ListSortDirection,
} from '../../../../../../../../../../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { VirtualizedList } from '../../../../../../../../../../../shared/defguard-ui/components/Layout/VirtualizedList/VirtualizedList';
import { Connection } from '../../../../../../../../../types';

type Props = {
  connections: Connection[];
};

const getDuration = (start: string, end: string): string => {
  const minutes = dayjs.utc(end).diff(dayjs.utc(start), 'm');
  if (minutes > 60) {
    const hours = floor(minutes / 60);
    const res = [`${hours}h`];
    if (minutes % 60 > 0) {
      res.push(`${minutes % 60}m`);
    }
    return res.join(' ');
  } else {
    return `${minutes}m`;
  }
};

export const LocationHistoryTable = ({ connections }: Props) => {
  const { LL } = useI18nContext();
  const pageLL = LL.pages.client.pages.instancePage.detailView.history.headers;
  const connectionsLength = useRef(0);
  const listHeaders = useMemo((): ListHeader[] => {
    return [
      {
        text: pageLL.date(),
        key: 'date',
        sortDirection: ListSortDirection.DESC,
        sortable: true,
        active: true,
      },
      {
        text: pageLL.duration(),
        key: 'duration',
      },
      {
        text: pageLL.upload(),
        key: 'upload',
      },

      {
        text: pageLL.download(),
        key: 'download',
      },
    ];
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const listCells = useMemo((): ListRowCell<Connection>[] => {
    const allCells = [
      {
        key: 'date',
        render: (connection: Connection) => (
          <span className="date">
            {`${dayjs.utc(connection.start).local().format('DD.MM.YYYY')} :
            ${dayjs.utc(connection.start).local().format('HH.mm')} -
            ${dayjs.utc(connection.end).local().format('HH.mm')} `}
          </span>
        ),
      },
      {
        key: 'duration',
        render: (connection: Connection) => (
          <span className="duration">
            {`${getDuration(connection.start, connection.end)}`}
          </span>
        ),
      },
      {
        key: 'connected_from',
        render: (connection: Connection) => (
          <span className="connected-from">{connection.connected_from}</span>
        ),
      },
      {
        key: 'upload',
        render: (connection: Connection) => (
          <span className="upload">{`${byteSize(connection.upload ?? 0)}`}</span>
        ),
      },

      {
        key: 'download',
        render: (connection: Connection) => (
          <span className="download">{`${byteSize(connection.download ?? 0)}`}</span>
        ),
      },
    ];
    return allCells;
  }, []);

  const rowRender = useCallback(
    (data: Connection, index?: number): ReactNode => {
      return (
        <div
          className={classNames('custom-row', {
            last: !isUndefined(index) && index === connectionsLength.current,
          })}
          data-index={index}
        >
          {listCells.map((cell, cellIndex) => (
            <div className={`cell-${cellIndex}`} key={cellIndex}>
              {cell.render(data)}
            </div>
          ))}
        </div>
      );
    },
    [listCells],
  );

  const getListPadding = useMemo(() => {
    return {
      left: 25,
      right: 25,
    };
  }, []);

  useEffect(() => {
    connectionsLength.current = connections.length;
  }, [connections]);

  return (
    <VirtualizedList
      className="connections-list"
      rowSize={40}
      data={connections}
      headers={listHeaders}
      cells={listCells}
      padding={getListPadding}
      customRowRender={rowRender}
    />
  );
};
