/* eslint-disable no-irregular-whitespace */
/* eslint-disable max-len */
import { deepmerge } from 'deepmerge-ts';

import en from '../en';
import type { BaseTranslation } from '../i18n-types';

const translation = {
  time: {
    seconds: {
      singular: '초',
      plural: '초',
    },
    minutes: {
      singular: '분',
      plural: '분',
    },
  },
  form: {
    errors: {
      invalid: '필드가 유효하지 않습니다',
      email: '유효한 이메일을 입력하세요',
      required: '필수 필드입니다',
      minValue: '필드는 최소 {min: number}의 값이 필요합니다',
      maxValue: '필드는 최대 {max: number}의 값을 초과할 수 없습니다',
      aboveZero: '필드 값은 0보다 커야 합니다',
      minLength: '최소 길이 {length: number}',
      maxLength: '최대 길이 {length: number}',
      specialsRequired: '최소 하나 이상의 특수 문자 필요',
      specialsForbidden: '특수 문자는 금지됩니다',
      numberRequired: '최소 하나 이상의 숫자 필요',
      password: {
        floatingTitle: '다음을 수정해주세요:',
      },
      oneLower: '최소 하나 이상의 소문자 필요',
      oneUpper: '최소 하나 이상의 대문자 필요',
      duplicatedName: '이 이름을 가진 장치가 이미 존재합니다',
    },
  },
  common: {
    controls: {
      back: '뒤로',
      next: '다음',
      submit: '제출',
      cancel: '취소',
      close: '닫기',
      reset: '초기화',
      save: '저장',
    },
    messages: {
      error: '예상치 못한 오류가 발생했습니다!',
      errorWithMessage: '오류 발생: {message}',
      tokenExpired:
        '토큰이 만료되었습니다. 관리자에게 문의하여 새 등록 토큰을 발급받으세요',
      networkError: '네트워크 오류가 발생했습니다. 프록시에 연결할 수 없습니다.',
      configChanged:
        '{instance: string} 인스턴스의 구성이 변경되었습니다. 변경 사항을 적용하려면 모든 위치에서 연결을 해제하세요.',
      deadConDropped:
        '{con_type: string} {interface_name: string} 연결이 끊어진 것을 감지했습니다. 재연결을 시도합니다...',
      noCookie: 'defguard_proxy set-cookie를 받지 못했습니다',
    },
  },
  components: {
    adminInfo: {
      title: '관리자 정보',
    },
  },
  pages: {
    client: {
      modals: {
        deadConDropped: {
          title: '{conType: string} {name: string} 연결 끊김',
          tunnel: '터널',
          location: '위치',
          message:
            '{conType: string} {name: string} 연결이 끊어졌습니다. 서버가 {time: number}초 동안 트래픽 응답을 하지 않는 것을 감지했습니다. 이 메시지가 계속 발생하면 관리자에게 이 사실을 알리십시오.',
          controls: {
            close: '닫기',
          },
        },
      },
      pages: {
        carouselPage: {
          slides: {
            shared: {
              //md
              isMore: '**defguard**는 이 모든 것 이상입니다!',
              githubButton: 'defguard 방문하기',
            },
            welcome: {
              // md
              title: '**defguard** 데스크톱 클라이언트에 오신 것을 환영합니다!',
              instance: {
                title: '인스턴스 추가',
                subtitle:
                  '단일 토큰으로 구성하여 defguard 인스턴스에 손쉽게 연결하세요.',
              },
              tunnel: {
                title: '터널 추가',
                subtitle:
                  'WireGuard® 데스크톱 클라이언트로 쉽게 활용하세요. 자체 터널을 설정하거나 구성 파일을 가져오세요.',
              },
            },
            twoFa: {
              // md
              title: 'defguard를 사용한 WireGuard **2단계 인증**',
              // md
              sideText: `WireGuard 프로토콜은 2FA/MFA를 지원하지 않으므로 현재 사용 가능한 대부분(전부는 아닐지라도)의 WireGuard 클라이언트는 실제 다중 요소 인증/2FA를 지원하지 않으며, 2FA를 "애플리케이션" 자체(WireGuard 터널이 아님)에 대한 인증으로만 사용합니다.

WireGuard 인스턴스를 보호하려면 **defguard** VPN & SSO 서버(무료 및 오픈 소스)를 사용하여 WireGuard PSK 키와 defguard 게이트웨이를 통한 피어 구성을 사용하여 실제 2FA를 구현해보세요!`,
            },
            security: {
              // md
              title: '보안 및 개인 정보 보호, **제대로 구현!**',
              // md
              sideText: `* 개인 정보 보호는 데이터 제어를 필요로 하므로 사용자 데이터(ID, SSO)는 온프레미스(사용자 서버)에 있어야 합니다.
* 데이터 및 애플리케이션 보안은 다중 요소 인증을 통한 인증 및 권한 부여(SSO)를 필요로 하며, 최고 수준의 보안을 위해서는 하드웨어 보안 모듈을 사용한 MFA가 필요합니다.
* 데이터 및 애플리케이션에 안전하고 비공개적으로 액세스하려면 데이터 암호화(HTTPS)와 모든 트래픽을 암호화하기 위한 장치와 인터넷 간의 보안 터널(VPN)이 필요합니다.
* SSO, VPN을 완전히 신뢰하려면 오픈 소스여야 합니다.`,
            },
            instances: {
              // md
              title: '**다중** 인스턴스 및 위치',
              // md
              sideText: `**defguard**(서버 및 이 클라이언트 모두)는 다중 인스턴스(설치) 및 다중 위치(VPN 터널)을 지원합니다.

관리자/데브옵스라면 모든 고객(인스턴스)과 모든 터널(위치)을 한 곳에서 관리할 수 있습니다!`,
            },
            support: {
              // md
              title: 'Github에서 **저희를 지원해주세요**',
              // md
              text: `**defguard**는 무료이며 진정한 오픈 소스이며 저희 팀은 몇 달 동안 작업해 왔습니다. 다음을 통해 저희를 지원하는 것을 고려해주세요: `,
              githubText: `다음을 통해 저희에게 별을 주세요`,
              githubLink: `GitHub`,
              spreadWordText: `다음에 대한 소문을 퍼뜨려주세요:`,
              defguard: `defguard!`,
              joinMatrix: `저희 Matrix 서버에 참여하세요:`,
              supportUs: '저희를 지원해주세요!',
            },
          },
        },
        settingsPage: {
          title: '설정',
          tabs: {
            global: {
              common: {
                value_in_seconds: '(초)',
              },
              peer_alive: {
                title: '세션 타임아웃',
                helper:
                  '활성 연결이 서버와 핸드셰이크를 하지 않고 주어진 시간을 초과하면 연결이 유효하지 않은 것으로 간주되어 자동으로 연결이 끊어집니다.',
              },
              tray: {
                title: '시스템 트레이',
                label: '트레이 아이콘 테마',
                options: {
                  color: '컬러',
                  white: '흰색',
                  black: '검은색',
                  gray: '회색',
                },
              },
              logging: {
                title: '로깅 임계값',
                warning: '변경 사항은 클라이언트 재시작 후 적용됩니다.',
                options: {
                  error: '오류',
                  info: '정보',
                  debug: '디버그',
                  trace: '추적',
                },
              },
              globalLogs: {
                logSources: {
                  client: '클라이언트',
                  service: '서비스',
                  all: '모두',
                },
                logSourceHelper:
                  '로그 소스입니다. 로그는 Defguard 클라이언트 또는 네트워크 수준에서 VPN 연결을 관리하는 백그라운드 Defguard 서비스에서 올 수 있습니다.',
              },
              theme: {
                title: '테마',
                options: {
                  light: '라이트',
                  dark: '다크',
                },
              },
              versionUpdate: {
                title: '업데이트',
                checkboxTitle: '업데이트 확인',
              },
            },
          },
        },
        createdPage: {
          tunnel: {
            title: '터널이 성공적으로 추가되었습니다',
            content:
              '터널이 성공적으로 추가되었습니다. 이제 이 장치를 연결하고 상태를 확인하며 왼쪽 사이드바 메뉴를 사용하여 통계를 볼 수 있습니다.',
            controls: {
              submit: '다른 터널 추가',
            },
          },
          instance: {
            title: '인스턴스가 성공적으로 추가되었습니다',
            content:
              '인스턴스가 성공적으로 추가되었습니다. 이제 이 장치를 연결하고 상태를 확인하며 왼쪽 사이드바 메뉴를 사용하여 통계를 볼 수 있습니다.',
            controls: {
              submit: '다른 인스턴스 추가',
            },
          },
        },
        instancePage: {
          title: '위치',
          controls: {
            connect: '연결',
            disconnect: '연결 해제',
            traffic: {
              predefinedTraffic: '사전 정의된 트래픽',
              allTraffic: '모든 트래픽',
              label: '허용된 트래픽',
              helper: `
                <p>
                  <b>사전 정의된 트래픽</b> - 관리자가 정의한 네트워크 트래픽만 이 VPN 위치를를 통해 라우팅합니다</br>
                  <b>모든 트래픽</b> - 모든 네트워크 트래픽을 이 VPN 위치를를 통해 라우팅 합니다
                </p>`,
            },
          },
          header: {
            title: '위치',
            edit: '인스턴스 편집',
            filters: {
              views: {
                grid: '그리드 뷰',
                detail: '상세 뷰',
              },
            },
          },
          connectionLabels: {
            lastConnectedFrom: '마지막 연결 위치',
            lastConnected: '마지막 연결',
            connectedFrom: '연결 위치',
            assignedIp: '할당된 IP',
            active: '활성',
            neverConnected: '연결된 적 없음',
          },
          locationNeverConnected: {
            title: '연결된 적 없음',
            content:
              '이 장치는 이 위치에 연결된 적이 없습니다. 연결하여 통계 및 연결 정보를 확인하세요.',
          },
          LocationNoStats: {
            title: '통계 없음',
            content:
              '이 장치는 지정된 기간 동안 이 위치에 대한 통계가 없습니다. 위치에 연결하고 클라이언트가 통계를 수집할 때까지 기다리세요.',
          },
          detailView: {
            history: {
              title: '연결 기록',
              headers: {
                date: '날짜',
                duration: '기간',
                connectedFrom: '연결 위치',
                upload: '업로드',
                download: '다운로드',
              },
            },
            details: {
              title: '상세 정보',
              logs: {
                title: '로그',
              },
              info: {
                configuration: {
                  title: '장치 구성',
                  pubkey: '공개 키',
                  address: '주소',
                  listenPort: '수신 포트',
                },
                vpn: {
                  title: 'VPN 서버 구성',
                  pubkey: '공개 키',
                  serverAddress: '서버 주소',
                  allowedIps: '허용된 IP',
                  dns: 'DNS 서버',
                  keepalive: '지속적 Keepalive',
                  handshake: '최신 핸드셰이크',
                  handshakeValue: '{seconds: number}초 전',
                },
              },
            },
          },
        },
        tunnelPage: {
          title: 'WireGuard 터널',
          header: {
            edit: '터널 편집',
          },
        },

        editTunnelPage: {
          title: 'WireGuard® 터널 편집',
          messages: {
            editSuccess: '터널 편집됨',
            editError: '터널 편집 실패',
          },
          controls: {
            save: '변경 사항 저장',
          },
        },
        addTunnelPage: {
          title: 'WireGuard® 터널 추가',
          forms: {
            initTunnel: {
              title: '인스턴스 URL과 토큰을 제공해주세요',
              sections: {
                vpnServer: 'VPN 서버',
                advancedOptions: '고급 옵션',
              },
              labels: {
                name: '터널 이름',
                privateKey: '개인 키',
                publicKey: '공개 키',
                address: '주소',
                serverPubkey: '공개 키',
                presharedKey: '사전 공유 키',
                endpoint: 'VPN 서버 주소:포트',
                dns: 'DNS',
                allowedips: '허용된 IP (쉼표로 구분)',
                persistentKeepAlive: '지속적 Keep Alive (초)',
                preUp: 'PreUp',
                postUp: 'PostUp',
                PreDown: 'PreDown',
                PostDown: 'PostDown',
              },
              helpers: {
                advancedOptions:
                  '"고급 옵션" 섹션을 클릭하여 WireGuard 터널 구성을 미세 조정하기 위한 추가 설정을 확인하세요. 다른 옵션 중에서 사전 및 사후 스크립트를 사용자 지정할 수 있습니다.',
                name: 'WireGuard 터널을 쉽게 식별할 수 있는 고유한 이름입니다.',
                pubkey:
                  '보안 통신을 위해 WireGuard 터널과 연결된 공개 키입니다.',
                prvkey:
                  '보안 통신을 위해 WireGuard 터널과 연결된 개인 키입니다.',
                address:
                  'VPN 네트워크 내에서 이 WireGuard 클라이언트에 할당된 IP 주소입니다.',
                serverPubkey:
                  '보안 통신을 위한 WireGuard 서버의 공개 키입니다.',
                presharedKey: '향상된 보안을 위한 선택적 대칭 비밀 키입니다.',
                allowedIps:
                  '터널을 통한 통신이 허용되는 IP 주소 또는 CIDR 범위의 쉼표로 구분된 목록입니다.',
                endpoint:
                  'WireGuard 서버의 주소 및 포트이며, 일반적으로 "호스트 이름:포트" 형식입니다.',
                dns: 'WireGuard 터널이 이름 확인에 사용해야 하는 DNS(도메인 이름 시스템) 서버입니다. 현재는 DNS 서버 IP만 지원하며, 향후 도메인 검색을 지원할 예정입니다.',
                persistentKeepAlive:
                  '터널이 활성 상태를 유지하도록 주기적인 keep-alive 메시지를 보내는 간격(초)입니다. 필요에 따라 조정하세요.',
                routeAllTraffic:
                  '활성화된 경우 모든 네트워크 트래픽이 WireGuard 터널을 통해 라우팅됩니다.',
                preUp:
                  'WireGuard 터널을 시작하기 전에 실행할 셸 명령 또는 스크립트입니다.',
                postUp:
                  'WireGuard 터널을 시작한 후 실행할 셸 명령 또는 스크립트입니다.',
                preDown:
                  'WireGuard 터널을 종료하기 전에 실행할 셸 명령 또는 스크립트입니다.',
                postDown:
                  'WireGuard 터널을 종료한 후 실행할 셸 명령 또는 스크립트입니다.',
              },
              submit: '터널 추가',
              messages: {
                configError: '구성 파일 구문 분석 오류',
                addSuccess: '터널 추가됨',
                addError: '터널 생성 실패',
              },
              controls: {
                importConfig: '구성 파일 가져오기',
                generatePrvkey: '개인 키 생성',
              },
            },
          },
          guide: {
            title: 'WireGuard 터널 추가',
            subTitle: `<p>인터넷을 통해 두 개 이상의 장치 간에 보안 통신을 설정하려면 터널을 구성하여 가상 사설망을 만듭니다.</p><p>Table 또는 MTU와 같은 옵션이 보이지 않으면 현재 지원하지 않지만 나중에 추가될 예정입니다.</p>`,
            card: {
              title: '새 터널 설정:',
              content: `
                <p>1. 구성 파일 가져오기</p>
                <div>
                <ul>
                  <li> "구성 파일 가져오기" 버튼을 클릭합니다.</li>
                  <li> 파일 선택 대화 상자를 사용하여 구성 파일로 이동합니다.</li>
                  <li> 받거나 생성한 .conf 파일을 선택합니다.</li>
                </ul>
                </div>
                <p>2. 또는 왼쪽 양식 작성</p>
                <div>
                <ul>
                  <li> 터널 이름을 입력합니다.</li>
                  <li> 개인 키, 공개 키 및 엔드포인트(서버 주소)와 같은 필수 세부 정보를 제공합니다.</li>
                </ul>
                </div>
                <p>
                자세한 내용은 defguard 도움말(https://docs.defguard.net)을 참조하세요.
                </p>
              `,
            },
          },
        },
        addInstancePage: {
          title: '인스턴스 추가',
          forms: {
            initInstance: {
              title: '인스턴스 URL과 토큰을 제공해주세요',
              labels: {
                url: '인스턴스 URL',
                token: '토큰',
              },
              submit: '인스턴스 추가',
            },
            device: {
              title: '이 장치 이름 지정',
              labels: {
                name: '이름',
              },
              submit: '완료',
              messages: {
                addSuccess: '장치 추가됨',
              },
            },
          },
          guide: {
            title: '인스턴스 추가 및 VPN 위치 연결',
            subTitle:
              '이 장치를 활성화하고 모든 VPN 위치에 액세스하려면 defguard 인스턴스 URL을 제공하고 활성화 토큰을 입력해야 합니다.',
            card: {
              title: '다음 방법으로 토큰을 얻을 수 있습니다',
              content: `
                <p>1. 원격 데스크톱 활성화 프로세스를 직접 호출</p>
                <div>
                <p>
                defguard 인스턴스에 액세스할 수 있는 경우(defguard에 액세스할 수 있는 집/사무실에 있는 경우), defguard -> 프로필 -> "장치 추가"로 이동하여 Defguard 클라이언트 활성화를 선택합니다. 그런 다음 토큰을 이메일로 받을지 아니면 defguard에서 복사할지 선택합니다.
                </p>
                </div>
                <p>2. 관리자가 원격으로 활성화</p>
                <div>
                <p>
                defguard에 액세스할 수 없는 경우 관리자에게 문의하여(온보딩 메시지/이메일에 관리자 연락처 정보가 있음) 원격 데스크톱 활성화를 요청하세요. 인스턴스 URL 및 토큰을 복사할 수 있는 활성화 이메일을 보내는 것이 가장 좋습니다.
                </p>
                </div>
                <p>
                자세한 내용은 defguard 도움말(https://docs.defguard.net)을 참조하세요.
                </p>
              `,
            },
          },
        },
      },
      sideBar: {
        instances: 'defguard 인스턴스',
        addInstance: '인스턴스 추가',
        addTunnel: '터널 추가',
        tunnels: 'WireGuard 터널',
        settings: '설정',
        copyright: {
          copyright: `Copyright © 2023`,
          appVersion: '애플리케이션 버전: {version:string}',
        },
        applicationVersion: '애플리케이션 버전: ',
      },
      newApplicationVersion: {
        header: '새 버전 사용 가능',
        dismiss: '닫기',
        releaseNotes: '새로운 기능 확인',
      },
    },
    enrollment: {
      sideBar: {
        title: '등록',
        steps: {
          welcome: '환영합니다',
          verification: '데이터 확인',
          password: '비밀번호 생성',
          vpn: 'VPN 구성',
          finish: '완료',
        },
        appVersion: '애플리케이션 버전',
      },
      stepsIndicator: {
        step: '단계',
        of: '/',
      },
      timeLeft: '남은 시간',
      steps: {
        welcome: {
          title: '안녕하세요, {name: string}님',
          explanation: `
회사 인프라에 액세스하려면 이 등록 프로세스를 완료해야 합니다. 이 프로세스 동안 다음을 수행해야 합니다.

1. 데이터 확인
2. 비밀번호 생성
3. VPN 장치 구성

이 프로세스를 완료하는 데 **{time: string}분**의 시간 제한이 있습니다.
질문이 있는 경우 할당된 관리자에게 문의하세요. 필요한 모든 정보는 사이드바 하단에서 찾을 수 있습니다.`,
        },
        dataVerification: {
          title: '데이터 확인',
          messageBox:
            '데이터를 확인해주세요. 잘못된 점이 있으면 프로세스를 완료한 후 관리자에게 알리세요.',
          form: {
            fields: {
              firstName: {
                label: '이름',
              },
              lastName: {
                label: '성',
              },
              email: {
                label: '이메일',
              },
              phone: {
                label: '전화번호',
              },
            },
          },
        },
        password: {
          title: '비밀번호 생성',
          form: {
            fields: {
              password: {
                label: '새 비밀번호 생성',
              },
              repeat: {
                label: '새 비밀번호 확인',
                errors: {
                  matching: `비밀번호가 일치하지 않습니다`,
                },
              },
            },
          },
        },
        deviceSetup: {
          desktopSetup: {
            title: '이 장치 구성',
            controls: {
              create: '장치 구성',
              success: '장치가 구성되었습니다',
            },
            messages: {
              deviceConfigured: '장치가 구성되었습니다',
            },
          },
          optionalMessage: `* 이 단계는 선택 사항입니다. 원하시면 건너뛸 수 있습니다. 나중에 defguard에서 구성할 수 있습니다.`,
          cards: {
            device: {
              title: 'VPN용 장치 구성',
              create: {
                submit: '구성 생성',
                messageBox:
                  '개인 키를 저장하지 않으므로 지금 구성을 다운로드해야 합니다. 이 대화 상자가 닫히면 전체 구성 파일(개인 키 포함, 빈 템플릿만)을 얻을 수 없습니다.',
                form: {
                  fields: {
                    name: {
                      label: '장치 이름',
                    },
                    public: {
                      label: '내 공개 키',
                    },
                    toggle: {
                      generate: '키 쌍 생성',
                      own: '내 공개 키 사용',
                    },
                  },
                },
              },
              config: {
                messageBox: {
                  auto: `
       <p>
          개인 키를 저장하지 않으므로 지금 구성을 다운로드해야 합니다.
          이 대화 상자가 닫히면 전체 구성 파일(개인 키 포함, 빈 템플릿만)을
          <strong>얻을 수 없습니다</strong>.
        </p>
`,
                  manual: `
        <p>
          여기에 제공된 구성에는 개인 키가 포함되어 있지 않으며 공개 키를 사용하여 해당 위치를 채웁니다. 구성이 제대로 작동하려면 직접 교체해야 합니다.
        </p>
`,
                },
                deviceNameLabel: '내 장치 이름',
                cardTitle:
                  'QR 코드를 스캔하거나 장치의 WireGuard 앱에서 파일로 가져와서 아래에 제공된 구성 파일을 사용하세요.',
                card: {
                  selectLabel: '위치용 구성 파일',
                },
              },
            },
            guide: {
              title: '빠른 가이드',
              messageBox: '이 빠른 가이드는 장치 구성에 도움이 될 것입니다.',
              step: '{step: number}단계:',
              steps: {
                wireguard: {
                  content:
                    '컴퓨터에 WireGuard 클라이언트를 다운로드하여 설치하거나 휴대폰에 앱을 설치하세요.',
                  button: 'WireGuard 다운로드',
                },
                downloadConfig: '제공된 구성 파일을 장치에 다운로드하세요.',
                addTunnel: `WireGuard를 열고 "터널 추가"(파일에서 터널 가져오기)를 선택합니다.
Defguard 구성 파일을 찾아 "확인"을 누릅니다. 휴대폰에서는 WireGuard 앱 "+" 아이콘을 사용하고 QR 코드를 스캔합니다.`,
                activate: '목록에서 터널을 선택하고 "활성화"를 누릅니다.',
                finish: `
**잘하셨습니다 - Defguard VPN이 이제 활성화되었습니다!**

VPN 연결을 해제하려면 "비활성화"를 누르기만 하면 됩니다.
`,
              },
            },
          },
        },
        finish: {
          title: '구성 완료!',
        },
      },
    },
    sessionTimeout: {
      card: {
        header: '세션 시간 초과',
        message:
          '죄송합니다. 프로세스를 완료하는 데 시간 제한을 초과했습니다. 다시 시도해주세요. 도움이 필요하면 가이드를 보거나 관리자에게 문의하세요.',
      },
      controls: {
        back: '새 토큰 입력',
        contact: '관리자에게 문의',
      },
    },
    token: {
      card: {
        title: '개인 등록 토큰을 입력해주세요',
        messageBox: {
          email: '이메일 메시지에서 토큰을 찾거나 직접 링크를 사용할 수 있습니다.',
        },
        form: {
          errors: {
            token: {
              required: '토큰이 필요합니다',
            },
          },
          fields: {
            token: {
              placeholder: '토큰',
            },
          },
          controls: {
            submit: '다음',
          },
        },
      },
    },
  },
  modals: {
    updateInstance: {
      title: '인스턴스 업데이트',
      infoMessage:
        "관리자가 보낸 토큰을 입력하여 인스턴스 구성을 업데이트하세요.\n또는 아래 '인스턴스 제거' 버튼을 클릭하여 이 인스턴스를 완전히 제거하도록 선택할 수 있습니다.",
      form: {
        fieldLabels: {
          token: '토큰',
          url: 'URL',
        },
        fieldErrors: {
          token: {
            rejected: '토큰 또는 URL이 거부되었습니다.',
            instanceIsNotPresent: '이 토큰에 대한 인스턴스를 찾을 수 없습니다.',
          },
        },
      },
      controls: {
        updateInstance: '인스턴스 업데이트',
        removeInstance: '인스턴스 제거',
      },
      messages: {
        success: '{name: string} 업데이트됨.',
        error: '토큰 또는 URL이 유효하지 않습니다.',
        errorInstanceNotFound: '주어진 토큰에 대한 인스턴스가 등록되지 않았습니다!',
      },
    },
    deleteInstance: {
      title: '인스턴스 삭제',
      subtitle: '{name: string}을(를) 삭제하시겠습니까?',
      messages: {
        success: '인스턴스 삭제됨',
        error: '예상치 못한 오류가 발생했습니다',
      },
      controls: {
        submit: '인스턴스 삭제',
      },
    },
    deleteTunnel: {
      title: '터널 삭제',
      subtitle: '{name: string}을(를) 삭제하시겠습니까?',
      messages: {
        success: '터널 삭제됨',
        error: '예상치 못한 오류가 발생했습니다',
      },
      controls: {
        submit: '터널 삭제',
      },
    },
    mfa: {
      authentication: {
        title: '2단계 인증',
        authenticatorAppDescription:
          '인증 앱에서 인증 코드를 붙여넣으세요.',
        emailCodeDescription:
          '이메일 주소로 전송된 인증 코드를 붙여넣으세요.',
        mfaStartDescriptionPrimary:
          '이 연결에는 2단계 인증(2FA)이 필수입니다.',
        mfaStartDescriptionSecondary: '선호하는 인증 방법을 선택하세요.',
        useAuthenticatorApp: '인증 앱 사용',
        useEmailCode: '이메일 코드 사용',
        saveAuthenticationMethodForFutureLogins: '향후 로그인을 위해 이 방법 사용',
        buttonSubmit: '확인',
        errors: {
          mfaNotConfigured: '선택한 방법이 구성되지 않았습니다.',
          mfaStartGeneric:
            'MFA 프로세스를 시작할 수 없습니다. 다시 시도하거나 관리자에게 문의하세요.',
          instanceNotFound: '인스턴스를 찾을 수 없습니다.',
          locationNotSpecified: '위치가가 지정되지 않았습니다.',
          invalidCode:
            '오류, 이 코드가 유효하지 않습니다. 다시 시도하거나 관리자에게 문의하세요.',
          tokenExpired: '토큰이 만료되었습니다. 다시 연결을 시도해주세요.',
        },
      },
    },
  },
} satisfies BaseTranslation;

const ko = deepmerge(en, translation);

export default ko;
