// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2024 The Gleam contributors

use super::*;
use insta::assert_snapshot;

// There are 2 separate tests for Windows and for Unix, because on Windows note
// about inability to create symlinks without Developer Mode is shown, so
// snapshots differ.

#[cfg(windows)]
#[test]
fn io_link_file_error_windows() {
    let error = Error::FileIo {
        kind: FileKind::File,
        action: FileIoAction::Link("/dest".into()),
        path: "/src".into(),
        cause: FileIoCause::Other("Critical error!".to_owned()),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[cfg(not(windows))]
#[test]
fn io_link_file_error_not_windows() {
    let error = Error::FileIo {
        kind: FileKind::File,
        action: FileIoAction::Link("/dest".into()),
        path: "/src".into(),
        cause: FileIoCause::Other("Critical error!".to_owned()),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[test]
fn io_copy_directory_error() {
    let error = Error::FileIo {
        kind: FileKind::Directory,
        action: FileIoAction::Copy(Some("/dest".into())),
        path: "/src".into(),
        cause: FileIoCause::Other("Critical error!".to_owned()),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[test]
fn io_delete_file_error() {
    let error = Error::FileIo {
        kind: FileKind::File,
        action: FileIoAction::Delete,
        path: "/file".into(),
        cause: FileIoCause::Other("Critical error!".to_owned()),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[test]
fn io_open_file_error() {
    let error = Error::FileIo {
        kind: FileKind::File,
        action: FileIoAction::Open,
        path: "/file".into(),
        cause: FileIoCause::Other("Critical error!".to_owned()),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[test]
fn io_parse_file_error() {
    let error = Error::FileIo {
        kind: FileKind::File,
        action: FileIoAction::Parse,
        path: "/file".into(),
        cause: FileIoCause::Other("Critical error!".to_owned()),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[test]
fn io_read_file_error() {
    let error = Error::FileIo {
        kind: FileKind::File,
        action: FileIoAction::Read,
        path: "/file".into(),
        cause: FileIoCause::Other("Critical error!".to_owned()),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[test]
fn io_create_directory_error() {
    let error = Error::FileIo {
        kind: FileKind::Directory,
        action: FileIoAction::Create,
        path: "/dir".into(),
        cause: FileIoCause::Other("Critical error!".to_owned()),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[test]
fn io_write_to_file_error() {
    let error = Error::FileIo {
        kind: FileKind::File,
        action: FileIoAction::WriteTo,
        path: "/file".into(),
        cause: FileIoCause::Other("Critical error!".to_owned()),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[test]
fn io_find_parent_of_directory_error() {
    let error = Error::FileIo {
        kind: FileKind::Directory,
        action: FileIoAction::FindParent,
        path: "/dir".into(),
        cause: FileIoCause::Other("Critical error!".to_owned()),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[test]
fn io_canonicalise_file_error() {
    let error = Error::FileIo {
        kind: FileKind::File,
        action: FileIoAction::Canonicalise,
        path: "/file".into(),
        cause: FileIoCause::Other("Critical error!".to_owned()),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[test]
fn io_update_file_permissions_error() {
    let error = Error::FileIo {
        kind: FileKind::File,
        action: FileIoAction::UpdatePermissions,
        path: "/file".into(),
        cause: FileIoCause::Other("Critical error!".to_owned()),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[test]
fn io_read_metadata_of_file_error() {
    let error = Error::FileIo {
        kind: FileKind::File,
        action: FileIoAction::ReadMetadata,
        path: "/file".into(),
        cause: FileIoCause::Other("Critical error!".to_owned()),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[test]
fn wrap_with_multibyte_char_at_width_boundary() {
    let input = format!("{}é", "a".repeat(74));
    let wrapped = wrap(&input);
    assert_eq!(wrapped.replace('\n', ""), input);
}

#[test]
fn lsp_message_send_failed() {
    let error = Error::LspMessageSendFailed {
        error: "sending on a disconnected channel".into(),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[test]
fn lsp_message_receive_failed() {
    let error = Error::LspMessageReceiveFailed {
        error: "disconnected channel".into(),
    }
    .pretty_string();
    assert_snapshot!(error);
}

#[test]
fn io_metadata_cache_error() {
    let error = Error::FileIo {
        kind: FileKind::File,
        action: FileIoAction::Parse,
        path: "wibble/wobble.cache_meta".into(),
        cause: FileIoCause::CacheMetadataFormatIncorrect,
    }
    .pretty_string();
    assert_snapshot!(error);
}
