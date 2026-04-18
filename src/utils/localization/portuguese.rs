use super::TextKey;

pub fn get_text(key: TextKey) -> &'static str {
    match key {
        TextKey::OperationLog => "Registro de Operações",
        TextKey::Contact => "Contato:",
        TextKey::CopyTelegram => "Copiar Link do Telegram",
        TextKey::CopyWeChat => "Copiar ID do WeChat",
        TextKey::CopyDiscord => "Copiar ID do Discord",
        TextKey::TelegramLink => "Link do Telegram",
        TextKey::WeChatID => "ID do WeChat",
        TextKey::DiscordID => "ID do Discord",
        TextKey::Copied => "Copiado:\n{}",
        TextKey::CheckingFiles => "Verificando arquivos...",
        TextKey::MissingFiles => "Arquivos Ausentes",
        TextKey::FileCheckSuccess => "Todos os arquivos validados com sucesso!",
        TextKey::SystemCheck => "Verificação do Sistema",
        TextKey::WelcomeMessage => "Bem-vindo à Ferramenta {}",
        TextKey::CheckingItem => "Verificando: {}",
        TextKey::CountdownMessage => "Continuando automaticamente em {} segundo{}...",
        TextKey::MissingFilesWarning => {
            "AVISO: Continuar sem os arquivos necessários pode causar erros"
        }
        TextKey::GroupExecutables => "Executáveis",
        TextKey::GroupLibraries => "Bibliotecas",
        TextKey::GroupBitstreams => "Bitstreams",
        TextKey::GroupConfigs => "Arquivos de Configuração",
        TextKey::GroupOther => "Outros Arquivos",
        TextKey::ContinueAnyway => "Continuar Mesmo Assim",
        TextKey::Rescan => "Reescanear Arquivos",
        TextKey::SelectOperation => "Selecionar Operação",
        TextKey::FlashFirmware => "Gravar Firmware",
        TextKey::ReadDna => "Ler DNA da Placa",
        TextKey::Drivers => "Drivers",
        TextKey::TestPcileech => "Test DMA (PCILeech)",
        TextKey::SelectFirmware => "Selecionar Arquivo de Firmware",
        TextKey::ScanningFirmware => "Procurando arquivos de firmware...",

        TextKey::NoFirmwareFound => "Nenhum arquivo de firmware encontrado no diretório atual",
        TextKey::PlaceFirmwareHere => {
            "Por favor, coloque arquivos de firmware .bin no diretório da aplicação"
        }
        TextKey::AutoScanning => "Escaneamento automático a cada 3 segundos",
        TextKey::AutoRefreshing => "Atualização automática",
        TextKey::PerformCleanup => "Realizar Limpeza",
        TextKey::CleanupDescription => {
            "(Excluir arquivo .bin de destino se a gravação for bem-sucedida)"
        }
        TextKey::Continue => "Continuar",
        TextKey::SelectFirmwareToContinue => "Selecione um arquivo de firmware para continuar",
        TextKey::FlashFirmwareDesc => "Gravar firmware no seu dispositivo",
        TextKey::ReadDnaDesc => "Recuperar o ID único do seu dispositivo",
        TextKey::DriversDesc => "Instalar drivers necessários para conexão ao dispositivo",
        TextKey::TestPcileechDesc => "Testar conexão DMA usando PCILeech",
        TextKey::DriversMenuTitle => "Instalar Drivers",
        TextKey::DataPortDrivers => "Drivers da Porta DATA",
        TextKey::JtagDrivers => "Drivers da Porta JTAG",
        TextKey::InstallFtdiDriver => "Instalar Driver FTDI",
        TextKey::OpenZadig => "Abrir Zadig (RS232)",
        TextKey::InstallCh347Driver => "Instalar Driver CH347",
        TextKey::RequiresAdmin => "Requer Administrador",
        TextKey::TestPcileechTitle => "Testar DMA (PCILeech)",
        TextKey::TestingConnection => "Testando conexão...",
        TextKey::TestSuccess => "Teste Bem-Sucedido",
        TextKey::TestFailed => "Teste Falhou",
        TextKey::ConnectionError => {
            "Soluções comuns:
1. Certifique-se de que o dispositivo FPGA está conectado
2. Verifique se o driver PCILeech está instalado
3. Execute como Administrador"
        }
        TextKey::SelectFlashingOption => "Selecionar Opção de Gravação",
        TextKey::SelectDnaReadOption => "Selecionar Opção de Leitura de DNA",
        TextKey::Ch347Options => "Opções CH347",
        TextKey::Rs232Options => "Opções RS232",

        TextKey::Ch347_35T_Label => "CH347 - 35T",
        TextKey::Ch347_35T_Desc => "Para placas 35T usando interface CH347",
        TextKey::Ch347_75T_Label => "CH347 - 75T",
        TextKey::Ch347_75T_Desc => "Para placas 75T usando interface CH347",
        TextKey::Ch347_100T_Label => "CH347 - 100T",
        TextKey::Ch347_100T_Desc => "Para placas 100T usando interface CH347",

        TextKey::Rs232_35T_Label => "RS232 - 35T",
        TextKey::Rs232_35T_Desc => "Para placas 35T usando interface RS232",
        TextKey::Rs232_75T_Label => "RS232 - 75T",
        TextKey::Rs232_75T_Desc => "Para placas 75T usando interface RS232",
        TextKey::Rs232_100T_Label => "RS232 - 100T",
        TextKey::Rs232_100T_Desc => "Para placas 100T usando interface RS232",

        TextKey::Dna_Ch347_Label => "CH347 - Leitura de DNA: 35T, 75T, 100T",
        TextKey::Dna_Ch347_Desc => "Ler DNA de 35T, 75T ou 100T usando interface CH347",
        TextKey::Dna_Rs232_35T_Label => "RS232 - Leitura de DNA: 35T",
        TextKey::Dna_Rs232_35T_Desc => "Ler DNA de placas 35T usando interface RS232",
        TextKey::Dna_Rs232_75T_Label => "RS232 - Leitura de DNA: 75T",
        TextKey::Dna_Rs232_75T_Desc => "Ler DNA de placas 75T usando interface RS232",
        TextKey::Dna_Rs232_100T_Label => "RS232 - Leitura de DNA: 100T",
        TextKey::Dna_Rs232_100T_Desc => "Ler DNA de placas 100T usando interface RS232",

        TextKey::ClearLog => "Limpar Registro",

        TextKey::OperationTook => "Operação levou",
        TextKey::NoteFewerSectors => {
            "Nota: Operação concluída com menos de 10 setores. Por favor, verifique manualmente ou tente novamente."
        }
        TextKey::NoteVerifySuccess => {
            "Nota: Não foi possível verificar o sucesso completo, mas nenhum erro foi detectado. Por favor, verifique manualmente ou tente novamente."
        }
        TextKey::ErrorDetails => "Detalhes do Erro",

        TextKey::Initializing => "Inicializando...",
        TextKey::StartingOperation => "Iniciando operação...",
        TextKey::WritingImage => "Gravando imagem na memória flash...",
        TextKey::ProbingFlash => "Examinando memória flash...",
        TextKey::ResettingFpga => "Reiniciando e interrompendo FPGA...",
        TextKey::LoadingBitstream => "Carregando bitstream...",
        TextKey::InitJtag => "Inicializando interface JTAG...",
        TextKey::Verifying => "Testando e verificando...",
        TextKey::WritingSector => "Gravando setor",
        TextKey::ReadingDeviceDna => "Lendo DNA do Dispositivo",
        TextKey::PleaseWaitDna => {
            "Por favor, aguarde enquanto recuperamos o ID único do seu dispositivo."
        }
        TextKey::DnaTakesSeconds => "Isso normalmente leva alguns segundos para concluir.",
        TextKey::FlashingFirmware => "Gravando Firmware",
        TextKey::PleaseWaitFlash => {
            "Por favor, aguarde enquanto o firmware está sendo gravado no seu dispositivo."
        }
        TextKey::FlashTakesMinutes => "Isso normalmente leva 1-2 minutos para concluir.",
        TextKey::FlashFailImmediate => {
            "Se o processo for concluído imediatamente, provavelmente falhou."
        }
        TextKey::TechnicalInfo => "Informações Técnicas",
        TextKey::InterfaceLabel => "Interface:",
        TextKey::OperationTypeLabel => "Tipo de Operação:",
        TextKey::TargetDeviceLabel => "Dispositivo de Destino:",

        TextKey::DnaReadSuccess => "LEITURA DE DNA BEM-SUCEDIDA!",
        TextKey::DnaReadFailed => "LEITURA DE DNA FALHOU",
        TextKey::DnaReadUnexpected => "STATUS DE LEITURA DE DNA INESPERADO",
        TextKey::DeviceDnaHeader => "DNA do Dispositivo",
        TextKey::ClickToCopy => "Clique para copiar",
        TextKey::FlashingSuccess => "GRAVAÇÃO BEM-SUCEDIDA!",
        TextKey::FlashingFailed => "GRAVAÇÃO FALHOU",
        TextKey::FlashingFailedConnection => "GRAVAÇÃO FALHOU - PROBLEMA DE CONEXÃO",
        TextKey::FlashingResultUnknown => "RESULTADO DA GRAVAÇÃO DESCONHECIDO",
        TextKey::NextSteps => "Próximos Passos",
        TextKey::NextStepsList => {
            "1. Reiniciar ambos os computadores\n2. Seguir os próximos passos no guia\n   - Instalar driver de firmware no computador host\n   - Trocar cabo para porta DATA\n   - Ativar usando software fornecido e código de ativação\n   - Construção de firmware bloqueados por DNA não requerem ativação"
        }
        TextKey::MainMenu => "Menu Principal",
        TextKey::TryAgainButton => "Tentar Novamente",

        TextKey::DnaReadUnexpectedMsg => {
            "A operação foi concluída, mas o valor do DNA não pôde ser confirmado.\nIsso pode indicar um problema com o processo de extração de DNA.\nPor favor, verifique a saída do registro para detalhes."
        }
        TextKey::DnaReadFailedPrefix => "Falha ao ler DNA do dispositivo:",
        TextKey::OperationInProgress => "Operação em andamento:",
        TextKey::DnaStatusUnknownMsg => {
            "Status da operação de leitura de DNA é desconhecido.\nPor favor, verifique o registro para detalhes."
        }
        TextKey::ClickToCopyTooltip => "Clique para copiar valores de DNA RAW, HEX e Verilog",
        TextKey::FlashingFailedConnectionMsg => {
            "Gravações normais de setor insuficientes detectadas: {} de {} setores.\n\nIsso indica um problema de conexão de hardware. O dispositivo está acessível, mas os dados não estão sendo transferidos corretamente.\n\nTente:\n1. Usar uma porta USB diferente\n2. Verificar as conexões de cabo\n3. Garantir que o dispositivo esteja alimentado corretamente\n4. Tentar um cabo USB diferente"
        }
        TextKey::FlashingResultUnknownMsg => {
            "Processo de gravação concluído, mas nenhuma informação de gravação de setor foi encontrada nos registros.\n\n1. Você selecionou o tipo de placa correto\n2. O driver USB apropriado está instalado e na porta JTAG\n3. Tente um cabo USB e/ou porta diferente\n4. Certifique-se de que o dispositivo esteja devidamente encaixado no slot PCIE."
        }
        TextKey::UnexpectedStateMsg => {
            "Este estado não deveria ser alcançado. Por favor, relate este bug."
        }
        TextKey::FlashingFailedPrefix => "Falha ao gravar firmware no dispositivo:",
        TextKey::FlashStatusUnknownMsg => {
            "Status da operação de gravação é desconhecido.\nPor favor, verifique o registro para detalhes ou tente novamente."
        }

        TextKey::DnaInvalidOption => "Opção inválida para leitura de DNA",
        TextKey::DnaCommandFailed => "Falha ao executar comando de leitura de DNA",
        TextKey::DnaFileNotFound => "Arquivo de saída de DNA não encontrado após {} tentativas",
        TextKey::DnaExtractFailed => "Falha ao extrair DNA: {}",
        TextKey::DnaFileReadError => "Falha ao ler arquivo de saída de DNA em {}: {}",
        TextKey::DnaInfoNotFound => {
            "Não foi possível encontrar informações de DNA no arquivo de saída"
        }
        TextKey::DnaWaitingStart => "Aguardando início da leitura de DNA...",
        TextKey::DnaRetrieving => "Recuperando DNA do dispositivo...",
        TextKey::DnaReadSuccessStatus => "Leitura de DNA bem-sucedida!",
        TextKey::DnaOperationCompleted => "Operação concluída (Não-DNA)",
        TextKey::DnaReadFailedStatus => "Leitura de DNA falhou: {}",
    }
}
