use bytes::Bytes;
use failure::Error;

use crate::mariadb::{Deserialize, ConnContext, Framed, Decoder, Message, Capabilities, DeContext, ColumnPacket, ColumnDefPacket, EofPacket, ErrPacket, OkPacket, ResultRow};

#[derive(Debug, Default)]
pub struct ResultSet {
    pub column_packet: ColumnPacket,
    pub columns: Vec<ColumnDefPacket>,
    pub rows: Vec<ResultRow>,
}

impl ResultSet {
    pub async fn deserialize<'a>(mut ctx: DeContext<'a>) -> Result<Self, Error> {
        let column_packet = ColumnPacket::deserialize(&mut ctx)?;

        let columns = if let Some(columns) = column_packet.columns {
            let mut column_defs = Vec::new();
            for _ in 0..columns {
                ctx.next_packet().await?;
                column_defs.push(ColumnDefPacket::deserialize(&mut ctx)?);
            }
            column_defs
        } else {
            Vec::new()
        };

        ctx.next_packet().await?;

        let eof_packet = if !ctx.ctx.capabilities.contains(Capabilities::CLIENT_DEPRECATE_EOF) {
            // If we get an eof packet we must update ctx to hold a new buffer of the next packet.
            let eof_packet = Some(EofPacket::deserialize(&mut ctx)?);
            ctx.next_packet().await?;
            eof_packet
        } else {
            None
        };

        ctx.columns = column_packet.columns.clone();

        let mut rows = Vec::new();

        loop {
            let packet_header = match ctx.decoder.peek_packet_header() {
                Ok(v) => v,
                Err(_) => break,
            };

            let tag = ctx.decoder.peek_tag();
            if tag == &0xFE && packet_header.length <= 0xFFFFFF || packet_header.length == 0 {
                break;
            } else {
                let index = ctx.decoder.index;
                match ResultRow::deserialize(&mut ctx) {
                    Ok(v) => {
                        rows.push(v);
                        ctx.next_packet().await?;
                    },
                    Err(_) => {
                        ctx.decoder.index = index;
                        break;
                    },
                }
            }
        }

        if ctx.decoder.peek_packet_header()?.length > 0 {
            if  ctx.ctx.capabilities.contains(Capabilities::CLIENT_DEPRECATE_EOF) {
                OkPacket::deserialize(&mut ctx)?;
            } else {
                EofPacket::deserialize(&mut ctx)?;
            }
        }

        Ok(ResultSet {
            column_packet,
            columns,
            rows
        })
    }
}

#[cfg(test)]
mod test {
    use bytes::{BufMut, Bytes};

    use crate::{__bytes_builder, mariadb::{Connection, EofPacket, ErrPacket, OkPacket, ResultRow, ServerStatusFlag, Capabilities, ConnContext}};
    use super::*;

    #[runtime::test]
    async fn it_decodes_result_set_packet() -> Result<(), Error> {
        // TODO: Use byte string as input for test; this is a valid return from a mariadb.
        #[rustfmt::skip]
        let buf = __bytes_builder!(
        // ------------------- //
        // Column Count packet //
        // ------------------- //
        // int<3> length
        1u8, 0u8, 0u8,
        // int<1> seq_no
        1u8,
        // int<lenenc> tag code or length
        4u8,

        // ------------------------ //
        // Column Definition packet //
        // ------------------------ //
        // int<3> length
        40u8, 0u8, 0u8,
        // int<1> seq_no
        2u8,
        // string<lenenc> catalog (always 'def')
        3u8, b"def",
        // string<lenenc> schema
        4u8, b"test",
        // string<lenenc> table alias
        5u8, b"users",
        // string<lenenc> table
        5u8, b"users",
        // string<lenenc> column alias
        2u8, b"id",
        // string<lenenc> column
        2u8, b"id",
        // int<lenenc> length of fixed fields (=0xC)
        0x0C_u8,
        // int<2> character set number
        8u8, 0u8,
        // int<4> max. column size
        0x80_u8, 0u8, 0u8, 0u8,
        // int<1> Field types
        0xFD_u8,
        // int<2> Field detail flag
        3u8, 64u8,
        // int<1> decimals
        0u8,
        // int<2> - unused -
        0u8, 0u8,

        // ------------------------ //
        // Column Definition packet //
        // ------------------------ //
        // int<3> length
        52u8, 0u8, 0u8,
        // int<1> seq_no
        3u8,
        // string<lenenc> catalog (always 'def')
        3u8, b"def",
        // string<lenenc> schema
        4u8, b"test",
        // string<lenenc> table alias
        5u8, b"users",
        // string<lenenc> table
        5u8, b"users",
        // string<lenenc> column alias
        8u8, b"username",
        // string<lenenc> column
        8u8, b"username",
        // int<lenenc> length of fixed fields (=0xC)
        0x0C_u8,
        // int<2> character set number
        8u8, 0u8,
        // int<4> max. column size
        0xFF_u8, 0xFF_u8, 0u8, 0u8,
        // int<1> Field types
        0xFC_u8,
        // int<2> Field detail flag
        0x11_u8, 0x10_u8,
        // int<1> decimals
        0u8,
        // int<2> - unused -
        0u8, 0u8,

        // ------------------------ //
        // Column Definition packet //
        // ------------------------ //
        // int<3> length
        52u8, 0u8, 0u8,
        // int<1> seq_no
        4u8,
        // string<lenenc> catalog (always 'def')
        3u8, b"def",
        // string<lenenc> schema
        4u8, b"test",
        // string<lenenc> table alias
        5u8, b"users",
        // string<lenenc> table
        5u8, b"users",
        // string<lenenc> column alias
        8u8, b"password",
        // string<lenenc> column
        8u8, b"password",
        // int<lenenc> length of fixed fields (=0xC)
        0x0C_u8,
        // int<2> character set number
        8u8, 0u8,
        // int<4> max. column size
        0xFF_u8, 0xFF_u8, 0u8, 0u8,
        // int<1> Field types
        0xFC_u8,
        // int<2> Field detail flag
        0x11_u8, 0x10_u8,
        // int<1> decimals
        0u8,
        // int<2> - unused -
        0u8, 0u8,

        // ------------------------ //
        // Column Definition packet //
        // ------------------------ //
        // int<3> length
        60u8, 0u8, 0u8,
        // int<1> seq_no
        5u8,
        // string<lenenc> catalog (always 'def')
        3u8, b"def",
        // string<lenenc> schema
        4u8, b"test",
        // string<lenenc> table alias
        5u8, b"users",
        // string<lenenc> table
        5u8, b"users",
        // string<lenenc> column alias
        0x0C_u8, b"access_level",
        // string<lenenc> column
        0x0C_u8, b"access_level",
        // int<lenenc> length of fixed fields (=0xC)
        0x0C_u8,
        // int<2> character set number
        8u8, 0u8,
        // int<4> max. column size
        7u8, 0u8, 0u8, 0u8,
        // int<1> Field types
        0xFE_u8,
        // int<2> Field detail flag
        1u8, 0x11_u8,
        // int<1> decimals
        0u8,
        // int<2> - unused -
        0u8, 0u8,

        // ---------- //
        // EOF Packet //
        // ---------- //
        // int<3> length
        5u8, 0u8, 0u8,
        // int<1> seq_no
        6u8,
        // int<1> 0xfe : EOF header
        0xFE_u8,
        // int<2> warning count
        0u8, 0u8,
        // int<2> server status
        34u8, 0u8,

        // ----------------- //
        // Result Row Packet //
        // ----------------- //
        // int<3> length
        62u8, 0u8, 0u8,
        // int<1> seq_no
        7u8,
        // string<lenenc> column data
        36u8, b"044d3f34-af65-11e9-a2e5-0242ac110003",
        // string<lenenc> column data
        4u8, b"josh",
        // string<lenenc> column data
        0x0B_u8, b"password123",
        // string<lenenc> column data
        7u8, b"regular",

        // ----------------- //
        // Result Row Packet //
        // ----------------- //
        // int<3> length
        52u8, 0u8, 0u8,
        // int<1> seq_no
        8u8,
        // string<lenenc> column data
        36u8, b"d83dd1c4-ada9-11e9-96bc-0242ac110003",
        // string<lenenc> column data
        6u8, b"daniel",
        // string<lenenc> column data
        1u8, b"f",
        // string<lenenc> column data
        5u8, b"admin",

        // ------------- //
        // OK/EOF Packet //
        // ------------- //
        // int<3> length
        5u8, 0u8, 0u8,
        // int<1> seq_no
        1u8,
        // 0xFE: Required header for last packet of result set
        0xFE_u8,
        // int<2> warning count
        0u8, 0u8,
        // int<2> server status
        34u8, 0u8
        );

        let mut context = ConnContext::new();
        let mut ctx = DeContext::new(&mut context, buf);

        ResultSet::deserialize(ctx).await?;

        Ok(())
    }
}
