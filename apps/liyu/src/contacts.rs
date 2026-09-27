//! Personal address book imports. No platform friendship is created.
use crate::data::ContactLocal;

pub fn normalize(kind: &str, value: &str) -> Option<String> {
    let value = value.trim();
    match kind {
        "phone" => {
            if value
                .chars()
                .any(|c| !c.is_ascii_digit() && !matches!(c, '+' | ' ' | '-' | '(' | ')'))
            {
                return None;
            }
            let compact: String = value
                .chars()
                .filter(|c| !matches!(c, ' ' | '-' | '(' | ')'))
                .collect();
            let compact = if compact.len() == 11 && compact.starts_with('1') {
                format!("+86{compact}")
            } else {
                compact
            };
            let d = compact.strip_prefix('+')?;
            (d.len() >= 8
                && d.len() <= 15
                && !d.starts_with('0')
                && d.bytes().all(|b| b.is_ascii_digit()))
            .then_some(compact)
        }
        "email" => {
            let (local, domain) = value.rsplit_once('@')?;
            if local.is_empty()
                || local.contains('@')
                || !domain.contains('.')
                || domain.starts_with('.')
                || domain.ends_with('.')
                || value.len() > 254
                || value.chars().any(|c| c.is_whitespace() || c.is_control())
            {
                return None;
            }
            Some(format!("{local}@{}", domain.to_ascii_lowercase()))
        }
        _ => None,
    }
}

#[derive(Default)]
pub struct Import {
    pub contacts: Vec<ContactLocal>,
    pub invalid: usize,
}

pub fn entry(label: &str, phones: &str, emails: &str) -> Result<ContactLocal, &'static str> {
    if label.trim().is_empty() || label.trim().chars().count() > 50 {
        return Err("称呼须为 1–50 字");
    }
    fn values(kind: &str, text: &str) -> Result<Vec<String>, &'static str> {
        let mut out = Vec::new();
        for v in text
            .split([';', ',', '\n'])
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let n = normalize(kind, v).ok_or("联系方式格式无效；国际手机号请带国家码")?;
            if !out.contains(&n) {
                out.push(n);
            }
        }
        Ok(out)
    }
    Ok(ContactLocal {
        id: 0,
        label: label.trim().into(),
        phones: Some(values("phone", phones)?),
        emails: Some(values("email", emails)?),
    })
}

pub fn choices(c: &ContactLocal) -> Vec<(String, String)> {
    c.phones
        .iter()
        .flatten()
        .map(|s| ("phone".into(), s.clone()))
        .chain(
            c.emails
                .iter()
                .flatten()
                .map(|s| ("email".into(), s.clone())),
        )
        .collect()
}

pub fn same_address(a: &ContactLocal, b: &ContactLocal) -> bool {
    let ac = choices(a);
    let bc = choices(b);
    if ac.is_empty() && bc.is_empty() {
        return a.label == b.label;
    }
    ac.iter().any(|v| bc.contains(v))
}

pub fn merge(target: &mut Vec<ContactLocal>, incoming: Vec<ContactLocal>) -> usize {
    let mut added = 0;
    for mut c in incoming {
        if let Some(prior) = target.iter_mut().find(|old| same_address(old, &c)) {
            for (kind, value) in choices(&c) {
                let values = if kind == "phone" {
                    prior.phones.get_or_insert_with(Vec::new)
                } else {
                    prior.emails.get_or_insert_with(Vec::new)
                };
                if !values.contains(&value) {
                    values.push(value);
                }
            }
        } else {
            c.id = target.iter().map(|v| v.id + 1).max().unwrap_or(0);
            target.push(c);
            added += 1;
        }
    }
    added
}

pub fn parse_vcard(text: &str) -> Import {
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        if (line.starts_with(' ') || line.starts_with('\t')) && !lines.is_empty() {
            lines.last_mut().unwrap().push_str(line.trim_start());
        } else {
            lines.push(line.trim_end_matches('\r').into());
        }
    }
    let mut result = Import::default();
    let mut block = Vec::new();
    let mut active = false;
    for line in lines {
        if line.eq_ignore_ascii_case("BEGIN:VCARD") {
            active = true;
            block.clear();
        } else if line.eq_ignore_ascii_case("END:VCARD") && active {
            let mut phones = Vec::new();
            let mut emails = Vec::new();
            for l in &block {
                let l: &String = l;
                if let Some((key, value)) = l.split_once(':') {
                    let key = key
                        .split(';')
                        .next()
                        .unwrap_or("")
                        .rsplit('.')
                        .next()
                        .unwrap_or("");
                    if key.eq_ignore_ascii_case("TEL") {
                        phones.push(value.strip_prefix("tel:").unwrap_or(value).to_string());
                    }
                    if key.eq_ignore_ascii_case("EMAIL") {
                        emails.push(value.to_string());
                    }
                }
            }
            let wrapped = format!("BEGIN:VCARD\n{}\nEND:VCARD", block.join("\n"));
            let names = crate::data::parse_vcard(&wrapped);
            let label = names.first().map(String::as_str).unwrap_or("");
            match entry(&unescape(label), &phones.join(";"), &emails.join(";")) {
                Ok(c) => result.contacts.push(c),
                Err(_) => result.invalid += 1,
            }
            active = false;
        } else if active {
            block.push(line);
        }
    }
    if active {
        result.invalid += 1;
    }
    result
}

fn unescape(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}

pub fn parse_csv(text: &str) -> Result<Import, &'static str> {
    let rows = csv_rows(text.trim_start_matches('\u{feff}'))?;
    let Some(header) = rows.first() else {
        return Err("CSV 文件为空");
    };
    let column = |names: &[&str]| {
        header
            .iter()
            .position(|s| names.iter().any(|n| s.trim().eq_ignore_ascii_case(n)))
    };
    let name = column(&["name", "display_name", "姓名", "称呼"])
        .ok_or("CSV 需有姓名/name 列及手机号/phone 或邮箱/email 列")?;
    let phone = column(&["phone", "mobile", "手机号", "电话"]);
    let email = column(&["email", "邮箱"]);
    if phone.is_none() && email.is_none() {
        return Err("CSV 缺少 phone/手机号 或 email/邮箱 列");
    }
    let mut result = Import::default();
    for row in rows.iter().skip(1) {
        if row.iter().all(|s| s.trim().is_empty()) {
            continue;
        }
        let field = |i: Option<usize>| i.and_then(|i| row.get(i)).map(String::as_str).unwrap_or("");
        match entry(field(Some(name)), field(phone), field(email)) {
            Ok(c) => result.contacts.push(c),
            Err(_) => result.invalid += 1,
        }
    }
    Ok(result)
}

fn csv_rows(text: &str) -> Result<Vec<Vec<String>>, &'static str> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            '"' if quoted => quoted = false,
            '"' if field.is_empty() => quoted = true,
            '"' => return Err("CSV 引号格式无效"),
            ',' if !quoted => row.push(std::mem::take(&mut field)),
            '\n' if !quoted => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            '\r' if !quoted => {}
            _ => field.push(c),
        }
    }
    if quoted {
        return Err("CSV 引号未闭合");
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

pub fn load_file(path: &std::path::Path) -> Result<Import, &'static str> {
    let meta = std::fs::metadata(path).map_err(|_| "无法读取文件")?;
    if meta.len() > 4 * 1024 * 1024 {
        return Err("联系人文件不能超过 4 MB");
    }
    let text = std::fs::read_to_string(path).map_err(|_| "请使用 UTF-8 编码的联系人文件")?;
    match path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "vcf" => Ok(parse_vcard(&text)),
        "csv" => parse_csv(&text),
        _ => Err("请选择 .vcf 或 .csv 文件"),
    }
}

pub fn system_contacts() -> Result<Import, &'static str> {
    #[cfg(target_os = "macos")]
    {
        let output=std::process::Command::new("osascript").args(["-e", "with timeout of 30 seconds\ntell application \"Contacts\"\nset resultText to \"\"\nrepeat with p in every person\nset resultText to resultText & vcard of p & linefeed\nend repeat\nreturn resultText\nend tell\nend timeout"]).output().map_err(|_|"无法打开系统通讯录，请从文件导入")?;
        if !output.status.success() {
            return Err("未获得通讯录访问权限，请允许访问或从文件导入");
        }
        let text = String::from_utf8(output.stdout).map_err(|_| "通讯录编码无效")?;
        Ok(parse_vcard(&text))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("此平台暂未接入系统通讯录，请从 vCard/CSV 文件导入")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn files_preserve_addresses_and_distinguish_same_names() {
        let file="BEGIN:VCARD\nFN:老陈\nTEL;TYPE=CELL:13800138000\nitem1.EMAIL:a+gift@EXAMPLE.COM\nEND:VCARD\nBEGIN:VCARD\nFN:老陈\nTEL:+14155550123\nEND:VCARD";
        let import = parse_vcard(file);
        assert_eq!(import.contacts.len(), 2);
        assert_eq!(choices(&import.contacts[0])[1].1, "a+gift@example.com");
        let mut contacts = Vec::new();
        assert_eq!(merge(&mut contacts, import.contacts), 2);
        assert_eq!(merge(&mut contacts, parse_vcard(file).contacts), 0);
        assert_eq!(contacts.len(), 2);
        let csv="name,phone,email\r\n\"陈,小明\",13800138000,\"a@example.com;b@example.com\"\r\n错误,abc,\r\n";
        let imported = parse_csv(csv).unwrap();
        assert_eq!(imported.invalid, 1);
        assert_eq!(imported.contacts[0].label, "陈,小明");
        assert_eq!(imported.contacts[0].emails.as_ref().unwrap().len(), 2);
        assert!(parse_csv("name,email\n\"unterminated,x@example.com").is_err());
    }
    #[test]
    fn old_contact_json_remains_readable() {
        use makepad_widgets::makepad_micro_serde::DeJson;
        let old = ContactLocal::deserialize_json("{\"id\":4,\"label\":\"旧联系人\"}").unwrap();
        assert!(choices(&old).is_empty());
    }
}
