import { Component, OnInit } from '@angular/core';
import { ApiService } from '../../../../@core/data/api.service';
import { NbToastrService, NbDialogService } from '@nebular/theme';
import { ConfirmDialogComponent } from '../../../../@core/components/confirm-dialog/confirm-dialog.component';

@Component({
  selector: 'app-tables',
  templateUrl: './tables.component.html',
  styleUrls: ['./tables.component.scss']
})
export class TablesComponent implements OnInit {

  tables: any[] = [];
  databases: any[] = [];
  loading = false;
  creating = false;
  editing = false;
  currentTable: any = null;
  newTable = {
    database: '',
    name: '',
    table_type: 'duplicate',
    columns: [
      {
          name: 'id', data_type: 'BIGINT', is_nullable: false, is_key: true, comment: '', default_value: '', aggregate_type: ''
        }
    ],
    partition_info: {
      partition_type: 'range',
      partition_key: '',
      partitions: []
    },
    bucket_info: {
      bucket_type: 'hash',
      bucket_keys: [],
      bucket_count: 10
    },
    comment: '',
    properties: {}
  };

  tableTypes = [
    { value: 'duplicate', label: '明细表' },
    { value: 'aggregate', label: '聚合表' },
    { value: 'unique', label: '唯一表' },
    { value: 'primary', label: '主键表' },
    { value: 'update', label: '更新表' }
  ];

  partitionTypes = [
    { value: 'range', label: '范围分区' },
    { value: 'list', label: '列表分区' },
    { value: 'expression', label: '表达式分区' }
  ];

  bucketTypes = [
    { value: 'hash', label: '哈希分桶' },
    { value: 'range', label: '范围分桶' }
  ];

  dataTypes = [
    'TINYINT', 'SMALLINT', 'INT', 'BIGINT',
    'FLOAT', 'DOUBLE', 'DECIMAL',
    'DATE', 'DATETIME', 'TIMESTAMP',
    'VARCHAR', 'CHAR', 'STRING',
    'BOOLEAN', 'ARRAY', 'MAP', 'STRUCT'
  ];

  aggregateTypes = [
    'SUM', 'COUNT', 'MAX', 'MIN', 'AVG', 'BITMAP_UNION', 'HLL_UNION'
  ];

  constructor(
    private apiService: ApiService,
    private toastrService: NbToastrService,
    private dialogService: NbDialogService
  ) { }

  ngOnInit(): void {
    this.loadDatabases();
    this.loadTables();
  }

  loadDatabases() {
    this.apiService.get('/clusters/databases').subscribe(
      (data: any[]) => {
        this.databases = data;
        if (data.length > 0) {
          this.newTable.database = data[0].name;
        }
      },
      error => {
        this.toastrService.danger('加载数据库失败', '错误');
      }
    );
  }

  loadTables() {
    this.loading = true;
    this.apiService.get('/clusters/tables').subscribe(
      (data: any[]) => {
        this.tables = data;
        this.loading = false;
      },
      error => {
        this.toastrService.danger('加载表失败', '错误');
        this.loading = false;
      }
    );
  }

  addColumn() {
    this.newTable.columns.push({
      name: '',
      data_type: 'VARCHAR',
      is_nullable: true,
      is_key: false,
      comment: '',
      default_value: '',
      aggregate_type: ''
    });
  }

  removeColumn(index: number) {
    if (this.newTable.columns.length > 1) {
      this.newTable.columns.splice(index, 1);
    }
  }

  addPartition() {
    this.newTable.partition_info.partitions.push({
      name: '',
      values: ''
    });
  }

  removePartition(index: number) {
    this.newTable.partition_info.partitions.splice(index, 1);
  }

  createTable() {
    if (!this.newTable.database || !this.newTable.name) {
      this.toastrService.warning('请输入数据库和表名称', '警告');
      return;
    }

    if (this.newTable.columns.length === 0) {
      this.toastrService.warning('请至少添加一个列', '警告');
      return;
    }

    this.creating = true;
    this.apiService.post('/clusters/tables', this.newTable).subscribe(
      (data: any) => {
        this.tables.push(data);
        this.toastrService.success('表创建成功', '成功');
        this.resetNewTable();
        this.creating = false;
      },
      error => {
        this.toastrService.danger('创建表失败', '错误');
        this.creating = false;
      }
    );
  }

  resetNewTable() {
    this.newTable = {
      database: this.databases.length > 0 ? this.databases[0].name : '',
      name: '',
      table_type: 'duplicate',
      columns: [
        { name: 'id', data_type: 'BIGINT', is_nullable: false, is_key: true, comment: '', default_value: '', aggregate_type: '' }
      ],
      partition_info: {
        partition_type: 'range',
        partition_key: '',
        partitions: []
      },
      bucket_info: {
        bucket_type: 'hash',
        bucket_keys: [],
        bucket_count: 10
      },
      comment: '',
      properties: {}
    };
  }

  editTable(table: any) {
    this.apiService.get(`/clusters/tables/${table.database}/${table.name}`).subscribe(
      (data: any) => {
        this.currentTable = data;
        this.editing = true;
      },
      error => {
        this.toastrService.danger('获取表详情失败', '错误');
      }
    );
  }

  updateTable() {
    if (!this.currentTable) return;

    this.apiService.put(`/clusters/tables/${this.currentTable.database}/${this.currentTable.name}`, {
      columns: this.currentTable.columns.map((col: any) => ({
        name: col.name,
        type: col.type,
        is_nullable: col.is_nullable,
        is_key: false,
        comment: col.comment,
        default_value: col.default_value,
        aggregate_type: ''
      })),
      comment: this.currentTable.comment
    }).subscribe(
      (data: any) => {
        const index = this.tables.findIndex(t => t.database === data.database && t.name === data.name);
        if (index !== -1) {
          this.tables[index] = data;
        }
        this.toastrService.success('表更新成功', '成功');
        this.currentTable = null;
        this.editing = false;
      },
      error => {
        this.toastrService.danger('更新表失败', '错误');
      }
    );
  }

  deleteTable(table: any) {
    this.dialogService.open(ConfirmDialogComponent, {
      context: {
        title: '删除表',
        message: `确定要删除表 ${table.database}.${table.name} 吗？此操作不可恢复。`,
        confirmText: '删除',
        cancelText: '取消'
      }
    }).onClose.subscribe(result => {
      if (result) {
        this.apiService.delete(`/clusters/tables/${table.database}/${table.name}`).subscribe(
          () => {
            this.tables = this.tables.filter(t => t.database !== table.database || t.name !== table.name);
            this.toastrService.success('表删除成功', '成功');
          },
          error => {
            this.toastrService.danger('删除表失败', '错误');
          }
        );
      }
    });
  }

  executeTableAction(table: any, action: string) {
    this.apiService.post(`/clusters/tables/${table.database}/${table.name}/action`, {
      action: action
    }).subscribe(
      () => {
        this.toastrService.success(`${action} 操作执行成功`, '成功');
      },
      error => {
        this.toastrService.danger(`${action} 操作执行失败`, '错误');
      }
    );
  }

  cancelEdit() {
    this.currentTable = null;
    this.editing = false;
  }

}
