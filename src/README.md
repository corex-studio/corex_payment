# Corex TTK2 - Rust библиотека для эквайринга и фискализации

Библиотека для работы с терминалами эквайринга и контрольно-кассовой техникой (ККТ).

## Основные компоненты

### Terminal (Эквайринг)

Класс для работы с платежными терминалами через протоколы TTK и Inpas.

#### Создание подключения

```rust
use corex_ttk2::{Terminal, ConnectionConfig, ProtocolType, ConnectionType};

let config = ConnectionConfig {
    connection_type: ConnectionType::Tcp,
    protocol: ProtocolType::Inpas,
    serial_number: "10285694".to_string(),
    address: Some("192.168.39.131".to_string()),
    port: Some(27015),
    timeout: Some(60),
    dc_host: Some("http://localhost:9015".to_string()),
    ncom: None,
    baudrate: None,
};

let mut terminal = Terminal::new(config);
```

#### Подключение

```rust
match terminal.connect().await {
    Ok(true) => println!("Подключено успешно"),
    Ok(false) => println!("Не удалось подключиться"),
    Err(e) => eprintln!("Ошибка подключения: {}", e),
}
```

#### Операции

**Оплата (Payment)**

```rust
let response = terminal.payment(10000, Some("643".to_string())).await?;
// amount - сумма в минимальных единицах валюты (10000 = 100.00 рублей)
// currency - код валюты (643 = RUB, по умолчанию)
```

**Возврат (Refund)**

```rust
let response = terminal.refund(5000, Some("643".to_string())).await?;
```

**Сверка итогов (Totals)**

```rust
let response = terminal.totals().await?;
```

#### Отключение

```rust
terminal.disconnect().await?;
```

#### Проверка статуса подключения

```rust
if terminal.connected() {
    println!("Терминал подключен");
}
```

### KKT (Контрольно-кассовая техника)

Класс для работы с ККТ через HTTP API.

#### Создание и настройка

```rust
use corex_ttk2::{Kkt, KktConfig, ConnectionType};

let config = KktConfig {
    connection_type: ConnectionType::Usb,
    address: None,
    port: None,
};

let mut kkt = Kkt::new(config);
```

#### Запуск сервера ККТ

```rust
kkt.run_server().await?;
```

#### Открытие смены

```rust
use corex_ttk2::kkt::Operator;

let operator = Operator {
    name: "Иванов Иван Иванович".to_string(),
    vatin: Some("123456789012".to_string()),
};

let response = kkt.open_shift(&operator).await?;
```

#### Закрытие смены

```rust
let response = kkt.close_shift(&operator).await?;
```

#### Продажа (Payment)

```rust
use corex_ttk2::kkt::{SellTask, Item, Payment, Tax};

let sell_task = SellTask {
    taxation_type: Some("osn".to_string()),
    electronically: true,
    operator: Some(operator),
    client_info: None,
    items: vec![
        Item {
            item_type: "position".to_string(),
            name: "Товар 1".to_string(),
            price: 100.0,
            quantity: 2.0,
            amount: 200.0,
            info_discount_amount: None,
            department: Some(1),
            measurement_unit: 0,
            payment_method: None,
            payment_object: None,
            tax: Some(Tax {
                tax_type: "vat20".to_string(),
            }),
        },
    ],
    payments: vec![Payment {
        payment_type: "electronically".to_string(),
        sum: 200.0,
    }],
    total: 200.0,
    taxes: None,
};

let response = kkt.payment(&sell_task).await?;
```

#### Возврат (Refund)

```rust
let response = kkt.refund(&sell_task).await?;
```

#### Получение документа

```rust
let response = kkt.document(123).await?;
```

#### Управление сервером

```rust
// Проверка, запущен ли сервер
let is_open = kkt.is_server_open().await?;

// Остановка сервера
kkt.stop_server().await?;
```

## Типы подключений

### ConnectionType

- `Tcp` - TCP/IP подключение
- `Usb` - USB подключение (последовательный порт)
- `Bluetooth` - Bluetooth подключение (не реализовано)

### ProtocolType

- `Ttk` - Протокол TTK (TLV формат, прямое подключение)
- `Inpas` - Протокол Inpas (XML формат через Dual Connector)

## Структура ответа TerminalResponse

```rust
pub struct TerminalResponse {
    pub success: bool,                    // Успешность операции
    pub code: Option<String>,             // Код ответа
    pub message: Option<String>,         // Текстовое сообщение
    pub data: Option<NormalizedTransactionData>, // Данные транзакции
    pub error: Option<String>,           // Описание ошибки (если есть)
}
```

### NormalizedTransactionData

Содержит нормализованные данные транзакции:

```rust
pub struct NormalizedTransactionData {
    pub message_id: Option<String>,           // ID сообщения
    pub operation_code: Option<String>,      // Код операции
    pub ecr_number: Option<String>,          // Номер ЭКЛЗ
    pub response_code: Option<String>,       // Код ответа
    pub approve: Option<String>,             // Одобрение (Y/N)
    pub status: Option<String>,              // Статус
    pub status_text: Option<String>,         // Текст статуса
    pub amount: Option<String>,              // Сумма транзакции
    pub additional_amount: Option<String>,    // Дополнительная сумма
    pub currency: Option<String>,            // Валюта
    pub pan_masked: Option<String>,          // Маскированный номер карты
    pub rrn: Option<String>,                 // Retrieval Reference Number
    pub invoice_number: Option<String>,      // Номер чека
    pub authorization_code: Option<String>,  // Код авторизации
    pub terminal_id: Option<String>,         // ID терминала
    pub merchant_id: Option<String>,         // ID мерчанта
    pub batch_number: Option<String>,        // Номер батча
    pub date: Option<String>,                // Дата
    pub time: Option<String>,               // Время
    pub timestamp: Option<String>,          // Временная метка
    pub host_timestamp: Option<String>,     // Временная метка хоста
    pub card_entry_mode: Option<String>,    // Режим ввода карты
    pub cardholder_verification: Option<String>, // Верификация держателя
    pub text_response: Option<String>,      // Текстовый ответ
    pub receipt: Option<String>,            // Чек
    pub application_label: Option<String>,  // Метка приложения
    pub issuer_name: Option<String>,        // Название эмитента
    pub transaction_id: Option<String>,     // ID транзакции
    pub cashier_request: Option<String>,    // Запрос кассира
    pub cashier_response: Option<String>,   // Ответ кассира
    pub provider_code: Option<String>,      // Код провайдера
    pub raw: HashMap<String, String>,       // Сырые данные
    pub extras: Option<HashMap<String, String>>, // Дополнительные поля
}
```

## Примеры использования

### Полный пример работы с Terminal

```rust
use corex_ttk2::{Terminal, ConnectionConfig, ProtocolType, ConnectionType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig {
        connection_type: ConnectionType::Tcp,
        protocol: ProtocolType::Inpas,
        serial_number: "10285694".to_string(),
        address: Some("192.168.39.131".to_string()),
        port: Some(27015),
        timeout: Some(60),
        dc_host: Some("http://localhost:9015".to_string()),
        ncom: None,
        baudrate: None,
    };

    let mut terminal = Terminal::new(config);
    
    // Подключение
    terminal.connect().await?;
    
    // Оплата 100 рублей
    let response = terminal.payment(10000, None).await?;
    
    if response.success {
        println!("Оплата успешна!");
        if let Some(data) = response.data {
            println!("Код авторизации: {:?}", data.authorization_code);
            println!("RRN: {:?}", data.rrn);
            println!("Чек: {:?}", data.receipt);
        }
    } else {
        println!("Ошибка: {:?}", response.error);
    }
    
    // Отключение
    terminal.disconnect().await?;
    
    Ok(())
}
```

### Пример работы с ККТ

```rust
use corex_ttk2::{Kkt, KktConfig, ConnectionType};
use corex_ttk2::kkt::{Operator, SellTask, Item, Payment, Tax};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = KktConfig {
        connection_type: ConnectionType::Usb,
        address: None,
        port: None,
    };

    let mut kkt = Kkt::new(config);
    
    // Запуск сервера
    kkt.run_server().await?;
    
    // Открытие смены
    let operator = Operator {
        name: "Иванов И.И.".to_string(),
        vatin: Some("123456789012".to_string()),
    };
    
    kkt.open_shift(&operator).await?;
    
    // Продажа
    let sell_task = SellTask {
        taxation_type: Some("osn".to_string()),
        electronically: true,
        operator: Some(operator.clone()),
        client_info: None,
        items: vec![
            Item {
                item_type: "position".to_string(),
                name: "Товар".to_string(),
                price: 100.0,
                quantity: 1.0,
                amount: 100.0,
                info_discount_amount: None,
                department: Some(1),
                measurement_unit: 0,
                payment_method: None,
                payment_object: None,
                tax: Some(Tax {
                    tax_type: "vat20".to_string(),
                }),
            },
        ],
        payments: vec![Payment {
            payment_type: "electronically".to_string(),
            sum: 100.0,
        }],
        total: 100.0,
        taxes: None,
    };
    
    let response = kkt.payment(&sell_task).await?;
    println!("Ответ ККТ: {:?}", response);
    
    // Закрытие смены
    kkt.close_shift(&operator).await?;
    
    Ok(())
}
```

## Обработка ошибок

Все методы возвращают `Result<T, Box<dyn std::error::Error>>`, поэтому ошибки нужно обрабатывать:

```rust
match terminal.payment(10000, None).await {
    Ok(response) => {
        if response.success {
            println!("Успех!");
        } else {
            eprintln!("Ошибка операции: {:?}", response.error);
        }
    }
    Err(e) => {
        eprintln!("Ошибка выполнения: {}", e);
    }
}
```

## Примечания

- Для протокола Inpas обязательно требуется указать `dc_host` в конфигурации
- Для TCP подключения в режиме Inpas требуются поля `address` и `port`
- Для USB подключения в режиме Inpas требуются поля `ncom` и `baudrate`
- Суммы передаются в минимальных единицах валюты (копейки для рублей)
- Все операции асинхронные и требуют использования `async/await`


## Возвраты Сбербанка: ошибки и данные попытки

`Terminal::refund(amount, currency)` сохраняет прежнюю сигнатуру. Сумма задаётся
в копейках (`39000` = 390 ₽). Это описание относится к `SBAdapter`, используемому
протоколами `Sb` и `Ttk`; обработка Inpas не меняется.

Библиотека запускает `sb_pilot.exe 3 <amount>` с рабочей директорией банковского
ПО и читает обновлённый файл `e` даже при ненулевом коде завершения процесса.
Код процесса и код терминала — разные значения: например, `99` и `4476`.
Код `0` процесса считается успехом возврата только вместе со свежим корректным
ответом терминала с кодом `0`.

При отказе возвращается `Err(ProcessError)`:

```json
{
  "message": "Возврат не выполнен: Нет исходной операции. Код терминала: 4476",
  "input_data": "39000",
  "raw_data": {
    "error": "Технические сведения о завершении процесса и чтении файла результата",
    "refund_attempt": {
      "attempt_id": "32 шестнадцатеричных символа",
      "operation": "refund",
      "amount_minor": 39000,
      "currency": "643",
      "started_at": "2026-11-12T09:49:00+00:00",
      "completed_at": "2026-11-12T09:49:22+00:00",
      "outcome": "declined",
      "terminal_response": "4476,Нет исходной операции\r\n",
      "terminal_result": { "result_code": 4476, "result_text": "Нет исходной операции" },
      "result_file_error": null,
      "process": {
        "started": true,
        "success": false,
        "exit_code": 99,
        "stdout": "",
        "stderr": "",
        "launch_error": null,
        "wait_error": null
      }
    }
  }
}
```

Пример сокращает `terminal_result`: фактически это сериализованный `SbPilotE`,
включающий его остальные поля. `terminal_response` содержит исходный текст
свежего файла `e`, декодированный как Windows-1251, без нормализации строк.
`stdout` и `stderr` декодируются как UTF-8, а при невалидном UTF-8 — Windows-1251.
`currency` сохраняет переданный аргумент либо `null`; команда Сбера, как и раньше,
его не использует. `attempt_id` — случайный идентификатор вызова библиотеки,
не идентификатор банковской транзакции и не ключ идемпотентности.

`refund_attempt` возвращается и при успехе, в `ProcessSuccess.raw_data`.
**Изменение формата:** для успешного возврата Сбера `raw_data` теперь объект с
`refund_attempt`, вместо строки Debug. `ProcessSuccess.data` остаётся
`NormalizedTransactionData`. Поля `message`, `raw_data`, `input_data` ошибки
сохранены; `raw_data.error` остаётся технической строкой.

Значения `outcome`:

- `approved` — процесс и терминал подтвердили успех.
- `declined` — свежий ответ терминала содержит положительный код ошибки.
- `not_started` — банковское ПО не удалось запустить.
- `unknown` — процесс запущен, но достоверный результат не получен либо результаты
  процесса и терминала противоречат друг другу. Перед повторным возвратом нужно
  проверить состояние операции: `Err` сам по себе не доказывает отсутствие списания.

Неизменившийся файл `e` не используется. Сравниваются содержимое, время изменения
и доступное время создания файла до и после запуска. Если свежесть проверить
невозможно, возвращается `unknown` с подробностями в `result_file_error`.
При грубой точности часов файловой системы повторный идентичный ответ может
также оказаться `unknown`. Банковские файлы библиотека не удаляет. Вызывающая
сторона должна выполнять операции одного терминала последовательно, в том числе
между разными экземплярами `Terminal` и процессами: файл `e` общий.

### Интеграция в Tauri и админку

Показывайте пользователю `ProcessError.message`, а технические поля сохраняйте
в логах. В обеих ветках результата (`Ok` и `Err`) сохраняйте
`raw_data.refund_attempt` вместе со связью с исходной оплатой и идентификатором
операции приложения. Библиотека не знает ID исходной оплаты, не обращается к
базе данных и сама не сохраняет метаданные в админку. Если требуется фиксировать
также аварийное завершение приложения до ответа библиотеки, создавайте запись
попытки на стороне Tauri до вызова `refund`.

Проверки библиотеки используют тестовый исполняемый файл на Unix и покрывают
коды 4476/362, код процесса 99, пустой/повреждённый/устаревший ответ, конфликт
результатов и ошибку запуска. Реальный обмен с `sb_pilot.exe` на Windows
не проверяется этими тестами. В предоставленных логах подтверждены ответы
терминала 4476/362, но файла `e` от этих попыток нет: его фактическое содержимое
при отказе нужно проверить на рабочем терминале. Если банковское ПО его не
обновляет, библиотека вернёт `unknown`, а не восстановит причину из COM-лога.
