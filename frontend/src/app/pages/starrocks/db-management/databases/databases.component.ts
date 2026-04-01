import { Component, OnInit } from '@angular/core';
import { ApiService } from '../../../../@core/data/api.service';
import { NbToastrService, NbDialogService } from '@nebular/theme';
import { ConfirmDialogComponent } from '../../../../@core/components/confirm-dialog/confirm-dialog.component';

@Component({
  selector: 'ngx-databases',
  templateUrl: './databases.component.html',
  styleUrls: ['./databases.component.scss']
})
export class DatabasesComponent implements OnInit {

  databases: any[] = [];
  loading = false;
  creating = false;
  editing = false;
  currentDatabase: any = null;
  newDatabase = {
    name: '',
    comment: '',
    properties: {}
  };

  constructor(
    private apiService: ApiService,
    private toastrService: NbToastrService,
    private dialogService: NbDialogService
  ) { }

  ngOnInit(): void {
    this.loadDatabases();
  }

  loadDatabases() {
    this.loading = true;
    this.apiService.get('/clusters/databases').subscribe(
      (data: any[]) => {
        this.databases = data;
        this.loading = false;
      },
      error => {
        this.toastrService.danger('加载数据库失败', '错误');
        this.loading = false;
      }
    );
  }

  createDatabase() {
    if (!this.newDatabase.name) {
      this.toastrService.warning('请输入数据库名称', '警告');
      return;
    }

    this.creating = true;
    this.apiService.post('/clusters/databases', this.newDatabase).subscribe(
      (data: any) => {
        this.databases.push(data);
        this.toastrService.success('数据库创建成功', '成功');
        this.newDatabase = {
          name: '',
          comment: '',
          properties: {}
        };
        this.creating = false;
      },
      error => {
        this.toastrService.danger('创建数据库失败', '错误');
        this.creating = false;
      }
    );
  }

  editDatabase(database: any) {
    this.currentDatabase = { ...database };
    this.editing = true;
  }

  updateDatabase() {
    if (!this.currentDatabase) return;

    this.apiService.put(`/clusters/databases/${this.currentDatabase.name}`, {
      comment: this.currentDatabase.comment,
      properties: this.currentDatabase.properties
    }).subscribe(
      (data: any) => {
        const index = this.databases.findIndex(d => d.name === data.name);
        if (index !== -1) {
          this.databases[index] = data;
        }
        this.toastrService.success('数据库更新成功', '成功');
        this.currentDatabase = null;
        this.editing = false;
      },
      error => {
        this.toastrService.danger('更新数据库失败', '错误');
      }
    );
  }

  deleteDatabase(database: any) {
    this.dialogService.open(ConfirmDialogComponent, {
      context: {
        title: '删除数据库',
        message: `确定要删除数据库 ${database.name} 吗？此操作不可恢复。`,
        confirmText: '删除',
        cancelText: '取消'
      }
    }).onClose.subscribe(result => {
      if (result) {
        this.apiService.delete(`/clusters/databases/${database.name}`).subscribe(
          () => {
            this.databases = this.databases.filter(d => d.name !== database.name);
            this.toastrService.success('数据库删除成功', '成功');
          },
          error => {
            this.toastrService.danger('删除数据库失败', '错误');
          }
        );
      }
    });
  }

  cancelEdit() {
    this.currentDatabase = null;
    this.editing = false;
  }

}
